use crate::internal_messages::messages::SendToKitchen;
use actix::fut::wrap_future;
use actix::prelude::*;
use common::logger::Logger;
use common::messages::{
    DeliverThisOrder, DeliveryAccepted, LeaderIs, NetworkMessage, NewOrder, RecoverProcedure,
    RegisterUser, RequestNearbyDelivery, UpdateOrderStatus, WhoIsLeader,
};
use common::network::communicator::Communicator;
use common::network::connections::{connect_some, try_to_connect};
use common::network::peer_types::PeerType;
use common::types::dtos::{OrderDTO, UserDTO};
use common::types::order_status::OrderStatus;
use common::types::restaurant_info::RestaurantInfo;
use common::utils::random_bool_by_given_probability;
use std::collections::HashSet;
use std::net::SocketAddr;
use tokio::net::TcpStream;

use crate::restaurant_actors::delivery_assigner::DeliveryAssigner;
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
            self.logger.error("Communicator not found!");
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
                position: self.info.position,
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

impl Handler<NewOrder> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: NewOrder, ctx: &mut Self::Context) -> Self::Result {
        let mut new_order: OrderDTO = msg.order;
        match new_order.status {
            OrderStatus::Pending => {
                self.logger.info(format!(
                    "Pending order detected: Client '{}' has an order for the dish '{}'.",
                    new_order.client_id, new_order.dish_name
                ));
                if let Some(kitchen_addr) = self.kitchen_address.clone() {
                    self.logger
                        .info(format!("Sending order {} to kitchen", new_order.dish_name));
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
                if random_bool_by_given_probability(self.probability) {
                    // Simulamos que el restaurante acepta el pedido
                    self.logger.info(format!(
                        "✅ Restaurant '{}' accepted order for client {} (dish: '{}')",
                        self.info.id, new_order.client_id, new_order.dish_name
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
                } else {
                    // Simulamos que el restaurante rechaza el pedido
                    self.logger.info(format!(
                        "❌ Restaurant {} rejected the order for client {} (dish: {})",
                        self.info.id, new_order.client_id, new_order.dish_name
                    ));
                    new_order.status = OrderStatus::Cancelled;
                    // Aquí podrías enviar un mensaje de rechazo al coordinador o al cliente
                }
                ctx.address().do_send(UpdateOrderStatus {
                    order: new_order.clone(),
                });
            }
            _ => {
                self.logger.warn(format!(
                    "Received new order with non-pending nor authorized status: {:?}",
                    new_order
                ));
            }
        }
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
        self.logger.info(format!(
            "Requesting nearby delivery for order ID: {}",
            msg.order.order_id
        ));
        self.send_network_message(NetworkMessage::RequestNearbyDelivery(msg));
    }
}

impl Handler<DeliveryAccepted> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: DeliveryAccepted, _ctx: &mut Self::Context) -> Self::Result {
        self.send_network_message(NetworkMessage::DeliveryAccepted(msg));
    }
}

impl Handler<DeliverThisOrder> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: DeliverThisOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.send_network_message(NetworkMessage::DeliverThisOrder(msg));
    }
}

impl Handler<NetworkMessage> for Restaurant {
    type Result = ();
    fn handle(&mut self, msg: NetworkMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            // All Users messages
            NetworkMessage::LeaderIs(msg_data) => ctx.address().do_send(msg_data),
            NetworkMessage::RecoveredInfo(user_dto_opt) => {
                let user_dto = user_dto_opt;
                match user_dto {
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
                }
            }
            NetworkMessage::NoRecoveredInfo => {
                self.logger
                    .info("No recovered info received, waiting for new orders.");
            }
            // Restaurant messages
            NetworkMessage::NewOrder(msg_data) => ctx.address().do_send(msg_data),
            NetworkMessage::UpdateOrderStatus(_msg_data) => {
                self.logger
                    .info("Received UpdateOrderStatus message, not implemented yet");
            }
            NetworkMessage::OrderFinalized(msg_data) => {
                self.logger.info(format!(
                    "Order finalized with ID: {}. Money transferred to restaurant.",
                    msg_data.order.order_id
                ));
            }
            NetworkMessage::DeliveryAvailable(_msg_data) => {
                if let Some(addr) = self.delivery_assigner_address.as_ref() {
                    addr.do_send(_msg_data);
                }
            }
            _ => {
                self.logger.info(format!(
                    "NetworkMessage received but not handled: {:?}",
                    msg
                ));
            }
        }
    }
}
