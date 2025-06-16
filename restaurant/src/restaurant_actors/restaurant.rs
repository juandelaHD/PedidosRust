use crate::internal_messages::messages::SendToKitchen;
use actix::fut::wrap_future;
use actix::prelude::*;
use common::logger::Logger;
use common::messages::{
    CancelOrder, DeliveryAccepted, LeaderIs, NetworkMessage, NewLeaderConnection, NewOrder, RecoverProcedure, RegisterUser, RequestNearbyDelivery, UpdateOrderStatus, WhoIsLeader
};
use common::network::communicator::{Communicator};
use common::network::connections::{connect_some, try_to_connect};
use common::network::peer_types::PeerType;
use common::types::dtos::{OrderDTO, UserDTO};
use common::types::order_status::OrderStatus;
use common::types::restaurant_info::RestaurantInfo;
use common::utils::random_bool_by_given_probability;
use std::collections::HashSet;
use std::net::SocketAddr;
use tokio::net::TcpStream;

use crate::restaurant_actors::delivery_assigner::{DeliveryAssigner};
use crate::restaurant_actors::kitchen::Kitchen;

pub struct Restaurant {
    /// Información básica del restaurante
    pub info: RestaurantInfo,
    /// Probabilidad de aceptar o rechazar un pedido.
    pub probability: f32,
    /// Canal de envío hacia la cocina.
    pub kitchen_address: Option<Addr<Kitchen>>,
    pub delivery_assigner_address: Option<Addr<DeliveryAssigner>>,
    pub communicator: Option<Communicator<Restaurant>>,
    pub pending_stream: Option<TcpStream>,
    pub logger: Logger,
}

impl Restaurant {
    pub async fn new(info: RestaurantInfo, probability: f32, servers: Vec<SocketAddr>) -> Self {
        let logger = Logger::new("Restaurant");
        logger.info(format!("Starting restaurant with ID: {}", info.id));
        // Intentamos conectarnos a los servidores
        let pending_stream = connect_some(servers.clone(), PeerType::RestaurantType).await;

        if pending_stream.is_none() {
            panic!("Failed to connect to any server. Try again later.");
        }
        Self {
            info,
            probability,
            kitchen_address: None,
            delivery_assigner_address: None,
            communicator: None,
            pending_stream,
            logger,
        }
    }

    pub fn send_network_message(&self, message: NetworkMessage) {
        if let Some(communicator) = &self.communicator {
            if let Some(sender) = &communicator.sender {
                sender.do_send(message);
            } else {
                self.logger.error("Sender not initialized in communicator");
            }
        } else {
            self.logger.error(&format!("Communicator not found!",));
        }
    }

    pub fn start_running(&self, _ctx: &mut Context<Self>) {
        let actual_socket_addr = self
            .communicator
            .as_ref()
            .map(|c| c.local_address)
            .expect("Socket address not initialized");
        self.send_network_message(NetworkMessage::WhoIsLeader(WhoIsLeader {
            origin_addr: actual_socket_addr,
            user_id: self.info.id.clone(),
        }));
    }
}

impl Actor for Restaurant {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let communicator = Communicator::new(
            self.pending_stream
                .take()
                .expect("Pending stream should be set"),
            ctx.address(),
            PeerType::ClientType,
        );
        self.communicator = Some(communicator);

        self.delivery_assigner_address =
            Some(DeliveryAssigner::new(self.info.clone(), ctx.address()).start());

        self.kitchen_address = Some(
                Kitchen::new(
                    ctx.address(),
                    self.delivery_assigner_address.clone().unwrap(),
                )
                .start(),
            );
        self.start_running(ctx);
    }
}

impl Handler<NewLeaderConnection> for Restaurant {
    type Result = ();

    fn handle(&mut self, _msg: NewLeaderConnection, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info("Received NewLeaderConnection, but dynamic communicator switching is not supported en este modelo".to_string());
    }
}

impl Handler<LeaderIs> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: LeaderIs, ctx: &mut Self::Context) -> Self::Result {
        let leader_addr = msg.coord_addr;
        let self_addr = ctx.address();
        let logger = self.logger.clone();

        let communicator_opt = self.communicator.as_ref().map(|c| c.peer_address);
        // Si ya estamos conectados al líder, no hacemos nada
        if Some(leader_addr) == communicator_opt {
            self.logger.info(format!(
                "Already connected to the leader at address: {}",
                leader_addr
            ));
            let local_address = self
                .communicator
                .as_ref()
                .map(|c| c.local_address)
                .expect("Socket address not set");
            self.send_network_message(NetworkMessage::RegisterUser(RegisterUser {
                origin_addr: local_address,
                user_id: self.info.id.clone(),
                position: self.info.position.clone(),
            }));
            return;
        }
        // Si no estamos conectados al líder, intentamos conectarnos
        ctx.spawn(wrap_future(async move {
            if let Some(new_stream) = try_to_connect(leader_addr).await {
                let new_communicator =
                    Communicator::new(new_stream, self_addr.clone(), PeerType::RestaurantType);
                self_addr.do_send(UpdateCommunicator(new_communicator));
                logger.info(format!(
                    "Communicator updated with new peer address: {}",
                    leader_addr
                ));
            } else {
                logger.error(format!(
                    "Failed to connect to the new leader at {}",
                    leader_addr
                ));
            }
        }));
        let actual_socket_addr = self
            .communicator
            .as_ref()
            .map(|c| c.local_address)
            .expect("Socket address not set");
        self.send_network_message(NetworkMessage::WhoIsLeader(WhoIsLeader {
            origin_addr: actual_socket_addr,
            user_id: self.info.id.clone(),
        }));
    }
}

impl Handler<RecoverProcedure> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: RecoverProcedure, ctx: &mut Self::Context) -> Self::Result {
        match msg.user_info {
            UserDTO::Restaurant(restaurant_dto) => {
                if restaurant_dto.restaurant_id == self.info.id {
                    self.logger.info(format!(
                        "Recovering info for Client ID={} ...",
                        restaurant_dto.restaurant_id
                    ));

                    self.info.position = restaurant_dto.restaurant_position;

                    // Dados dos sets, junta todos los pedidos en un set
                    let mut all_orders: HashSet<OrderDTO> = HashSet::new();
                    all_orders.extend(restaurant_dto.pending_orders);
                    all_orders.extend(restaurant_dto.authorized_orders);

                    for order in all_orders {
                        self.logger.info(format!(
                            "Recovered order with ID={} and status={:?}",
                            order.order_id, order.status
                        ));
                        ctx.address().do_send(NewOrder {
                            order: order.clone(),
                        });
                    }
                } else {
                    self.logger.warn(format!(
                        "Received recovered info for a different restaurant ({}), ignoring",
                        restaurant_dto.restaurant_id
                    ));
                }
            }
            other => {
                self.logger.warn(format!(
                    "Received recovered info of type {:?}, but I'm Client. Ignoring.",
                    other
                ));
            }
        }
    }
}

pub struct UpdateCommunicator(pub Communicator<Restaurant>);

impl Message for UpdateCommunicator {
    type Result = ();
}

impl Handler<UpdateCommunicator> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: UpdateCommunicator, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Updating communicator with new peer address: {}",
            msg.0.peer_address
        ));
        self.communicator = Some(msg.0);
    }
}

impl Handler<UpdateOrderStatus> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: UpdateOrderStatus, _ctx: &mut Self::Context) -> Self::Result {
        self.send_network_message(NetworkMessage::UpdateOrderStatus(msg));
    }
}

impl Handler<RequestNearbyDelivery> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: RequestNearbyDelivery, _ctx: &mut Self::Context) -> Self::Result {
        self.send_network_message(NetworkMessage::RequestNearbyDelivery(msg));
    }
}

impl Handler<DeliveryAccepted> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: DeliveryAccepted, _ctx: &mut Self::Context) -> Self::Result {
        self.send_network_message(NetworkMessage::DeliveryAccepted(msg));
    }
}

impl Handler<NetworkMessage> for Restaurant {
    type Result = ();
    fn handle(&mut self, msg: NetworkMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            // All Users messages
            NetworkMessage::WhoIsLeader(_msg_data) => {
                self.logger
                    .error("Received a WhoIsLeader message, handle not implemented");
            }
            NetworkMessage::LeaderIs(msg_data) => ctx.address().do_send(msg_data),
            NetworkMessage::RegisterUser(_msg_data) => {
                self.logger
                    .info("Received RegisterUser message, not implemented yet");
            }
            NetworkMessage::RecoveredInfo(user_dto_opt) => match user_dto_opt {
                user_dto => match user_dto {
                    UserDTO::Restaurant(restaurant_dto) => {
                        if restaurant_dto.restaurant_id == self.info.id {
                            self.logger.info(format!(
                                "Recovered info for Restaurant ID={}, updating local state...",
                                restaurant_dto.restaurant_id
                            ));
                            ctx.address().do_send(RecoverProcedure {
                                user_info: UserDTO::Restaurant(restaurant_dto),
                            });
                        } else {
                            self.logger.warn(format!(
                                "Received recovered info for a different delivery ({}), ignoring",
                                restaurant_dto.restaurant_id
                            ));
                        }
                    }
                    other => {
                        self.logger.warn(format!(
                            "Received recovered info of type {:?}, but I'm Delivery. Ignoring.",
                            other
                        ));
                    }
                },
            },
            NetworkMessage::NoRecoveredInfo => {
                self.logger
                    .info("No recovered info received, waiting for new orders.");
            }

            // Restaurant messages
            NetworkMessage::NewOrder(_msg_data) => ctx.address().do_send(_msg_data),

            NetworkMessage::UpdateOrderStatus(_msg_data) => {
                self.logger
                    .info("Received UpdateOrderStatus message, not implemented yet");
            }
            NetworkMessage::CancelOrder(_msg_data) => {
                self.logger
                    .info("Received CancelOrder message, not implemented yet");
            }
            NetworkMessage::DeliveryAvailable(_msg_data) => {
                self.delivery_assigner_address
                    .as_ref()
                    .map(|addr| addr.do_send(_msg_data));
            }
            NetworkMessage::DeliverThisOrder(_msg_data) => {
                self.logger
                    .info("Received DeliverThisOrder message, not implemented yet");
            }

            _ => {
                self.logger.info(format!(
                    "NetworkMessage descartado/no implementado: {:?}",
                    msg
                ));
            }
        }
    }
}

impl Handler<NewOrder> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: NewOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Received NewOrder message: {:?}", msg));
        // Aquí podrías implementar la lógica para manejar un nuevo pedido
        // Por ejemplo, enviar un mensaje a la cocina o actualizar el estado del restaurante
        let mut new_order: OrderDTO = msg.order;
        match new_order.status {
            OrderStatus::Pending => {
                self.logger
                    .info(format!("Order pending with ID: {}", new_order.order_id));
                if let Some(kitchen_addr) = self.kitchen_address.clone() {
                    self.logger
                        .info(format!("Sending order {} to kitchen", new_order.order_id));
                    // Enviamos el pedido a la cocina
                    kitchen_addr.do_send(SendToKitchen {
                        order: new_order.clone(),
                    });
                } else {
                    self.logger
                        .error("Kitchen sender is not set, cannot send order to kitchen");
                }
            }
            OrderStatus::Authorized => {
                self.logger
                    .info(format!("Order authorized with ID: {}", new_order.order_id));
                if random_bool_by_given_probability(self.probability) {
                    // Simulamos que el restaurante acepta el pedido
                    self.logger.info(format!(
                        "Restaurant {} accepted order {}",
                        self.info.id, new_order.order_id
                    ));
                    new_order.status = OrderStatus::Pending;
                    if let Some(kitchen_addr) = self.kitchen_address.clone() {
                        self.logger
                            .info(format!("Sending order {} to kitchen", new_order.order_id));
                        // Enviamos el pedido a la cocina
                        kitchen_addr.do_send(SendToKitchen {
                            order: new_order.clone(),
                        });
                    } else {
                        self.logger
                            .error("Kitchen sender is not set, cannot send order to kitchen");
                    }
                    self.send_network_message(NetworkMessage::UpdateOrderStatus(
                        UpdateOrderStatus {
                            order: new_order.clone(),
                        },
                    ));
                } else {
                    // Simulamos que el restaurante rechaza el pedido
                    self.logger.info(format!(
                        "Restaurant {} rejected order {}",
                        self.info.id, new_order.order_id
                    ));
                    new_order.status = OrderStatus::Cancelled;
                    self.send_network_message(NetworkMessage::CancelOrder(CancelOrder {
                        order: new_order,
                    }));
                    // Aquí podrías enviar un mensaje de rechazo al coordinador o al cliente
                }
            }
            _ => {
                self.logger.warn(format!(
                    "Received NewOrder with non-pending nor authorized status: {:?}",
                    new_order
                ));
            }
        }
    }
}
