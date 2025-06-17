use crate::client_actors::ui_handler::UIHandler;
use crate::messages::internal_messages::*;
use actix::fut::wrap_future;
use actix::prelude::*;
use common::logger::Logger;
use common::messages::NearbyRestaurants;
use common::messages::OrderDelivered;
use common::messages::client_messages::*;
use common::messages::shared_messages::*;
use common::network::communicator::Communicator;
use common::network::connections::{connect_some, try_to_connect};
use common::network::peer_types::PeerType;
use common::types::dtos::ClientDTO;
use common::types::dtos::OrderDTO;
use common::types::dtos::UserDTO;
use common::types::order_status::OrderStatus;
use rand::Rng;
use std::net::SocketAddr;
use tokio::net::TcpStream;

pub struct Client {
    /// Vector de direcciones de servidores
    pub servers: Vec<SocketAddr>,
    /// Identificador único del comensal.
    pub client_id: String,
    /// Posición actual del cliente en coordenadas 2D.
    pub client_position: (f32, f32),
    /// Restaurante elegido para el pedido.
    // pub selected_restaurant: Option<String>,
    /// Pedido actual.
    pub client_order: Option<OrderDTO>,
    /// Canal de envío hacia el actor `UIHandler`.
    pub ui_handler: Option<Addr<UIHandler>>,
    /// Comunicador asociado al `Server`.
    pub communicator: Option<Communicator<Client>>,
    pub pending_stream: Option<TcpStream>, // Guarda el stream hasta que arranque
    pub logger: Logger,
}

impl Client {
    pub async fn new(
        servers: Vec<SocketAddr>,
        client_id: String,
        client_position: (f32, f32),
    ) -> Self {
        let logger = Logger::new(format!("Client {}", &client_id));
        logger.info(format!("Starting client with ID: {}", client_id));
        // Intentamos conectarnos a los servidores
        let pending_stream = connect_some(servers.clone(), PeerType::ClientType).await;

        if pending_stream.is_none() {
            panic!("Failed to connect to any server. Try again later.");
        }

        Self {
            servers,
            client_id,
            client_position,
            client_order: None, // Inicializamos el pedido como None
            ui_handler: None,   // Inicializamos el canal de envío hacia UIHandler como None
            communicator: None,
            pending_stream, // Guarda el stream hasta que arranque
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
            user_id: self.client_id.clone(),
        }));
    }
}

impl Actor for Client {
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

        let ui_handler = UIHandler::new(ctx.address(), self.logger.clone());
        self.ui_handler = Some(ui_handler.start());

        self.start_running(ctx);
    }
}

pub struct UpdateCommunicator(pub Communicator<Client>);

impl Message for UpdateCommunicator {
    type Result = ();
}

impl Handler<UpdateCommunicator> for Client {
    type Result = ();

    fn handle(&mut self, msg: UpdateCommunicator, _ctx: &mut Self::Context) -> Self::Result {
        self.communicator = Some(msg.0);
    }
}

impl Handler<LeaderIs> for Client {
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
                user_id: self.client_id.clone(),
                position: self.client_position,
            }));
            return;
        }
        // Si no estamos conectados al líder, intentamos conectarnos
        ctx.spawn(wrap_future(async move {
            if let Some(new_stream) = try_to_connect(leader_addr).await {
                let new_communicator =
                    Communicator::new(new_stream, self_addr.clone(), PeerType::ClientType);
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
            user_id: self.client_id.clone(),
        }));
    }
}

impl Handler<RecoverProcedure> for Client {
    type Result = ();

    fn handle(&mut self, msg: RecoverProcedure, _ctx: &mut Self::Context) -> Self::Result {
        match msg.user_info {
            UserDTO::Client(client_dto) => {
                if client_dto.client_id == self.client_id {
                    self.logger.info(format!(
                        "Recovering info for Client ID={} ...",
                        client_dto.client_id
                    ));
                    self.client_position = client_dto.client_position;
                    self.client_order = client_dto.client_order;

                    // Si tengo una orden activa, chequeo su estado
                    if let Some(client_dto) = &self.client_order {
                        self.logger.info(format!(
                            "Client ID={} has an active order with status: {}",
                            client_dto.client_id, client_dto.status
                        ));

                        // Imprime los posibles estados del pedido
                        match client_dto.status {
                            OrderStatus::Cancelled => {
                                panic!("{}", client_dto.status)
                            }
                            OrderStatus::Delivered => {
                                panic!("Order delivered: {}", client_dto.dish_name);
                            }
                            _ => {
                                self.logger.info(format!("State: {}", client_dto.status));
                            }
                        }
                    } else {
                        // Si no tengo una orden activa, informo y solicito restaurantes cercanos
                        self.logger.info(format!(
                            "Client ID={} has no active order, requesting nearby restaurants.",
                            self.client_id
                        ));
                        let new_client_dto = ClientDTO {
                            client_position: self.client_position,
                            client_id: self.client_id.clone(),
                            client_order: None, // No hay orden activa
                            time_stamp: std::time::SystemTime::now(),
                        };
                        self.send_network_message(NetworkMessage::RequestNearbyRestaurants(
                            RequestNearbyRestaurants {
                                client: new_client_dto.clone(),
                            },
                        ));
                    }
                } else {
                    self.logger.warn(format!(
                        "Received recovered info for a different client ({}), ignoring",
                        client_dto.client_id
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

impl Handler<SendThisOrder> for Client {
    type Result = ();

    fn handle(&mut self, msg: SendThisOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Sending order to restaurant {}: {}",
            msg.selected_restaurant, msg.selected_dish
        ));

        let mut rng = rand::thread_rng();
        let order_id: u64 = rng.gen_range(1..=u64::MAX);

        let order = OrderDTO {
            order_id,
            client_id: self.client_id.clone(),
            restaurant_id: msg.selected_restaurant,
            dish_name: msg.selected_dish,
            status: OrderStatus::Pending, // Estado inicial del pedido
            delivery_id: None,            // No hay delivery asignado aún
            time_stamp: std::time::SystemTime::now(), // Marca de tiempo actual
            client_position: self.client_position, // Posición del cliente
        };

        // Enviar el pedido al servidor
        let network_message = NetworkMessage::RequestThisOrder(RequestThisOrder { order });
        self.send_network_message(network_message);
    }
}

impl Handler<NearbyRestaurants> for Client {
    type Result = ();

    fn handle(&mut self, msg: NearbyRestaurants, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Received NearbyRestaurants with {} restaurants",
            msg.restaurants.len()
        ));
        if let Some(ui_handler) = &self.ui_handler {
            ui_handler.do_send(SelectNearbyRestaurants {
                nearby_restaurants: msg.restaurants,
            });
        } else {
            self.logger.error("UIHandler not initialized");
        }
    }
}

impl Handler<NetworkMessage> for Client {
    type Result = ();
    fn handle(&mut self, msg: NetworkMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NetworkMessage::RetryLater(msg_data) => {
                self.logger.info(format!(
                    "Received RetryLater message: {:?}",
                    msg_data.origin_addr
                ));
            }
            // All Users messages
            NetworkMessage::LeaderIs(msg_data) => {
                self.logger.info(format!(
                    "Received LeaderIs message with addr: {}",
                    msg_data.coord_addr
                ));
                ctx.address().do_send(msg_data)
            }
            NetworkMessage::RecoveredInfo(user_dto) => match user_dto {
                UserDTO::Client(client_dto) => {
                    if client_dto.client_id == self.client_id {
                        ctx.address().do_send(RecoverProcedure {
                            user_info: UserDTO::Client(client_dto.clone()),
                        });
                    } else {
                        self.logger.warn(format!(
                            "Received recovered info for a different client ({}), ignoring",
                            client_dto.client_id
                        ));
                    }
                }
                other => {
                    self.logger.warn(format!(
                        "Received recovered info of type {:?}, but I'm Client. Ignoring.",
                        other
                    ));
                }
            },
            NetworkMessage::NoRecoveredInfo => {
                self.logger
                    .info("No recovered info received, proceeding with normal flow");
                // Aquí podrías enviar un mensaje para solicitar restaurantes cercanos
                let new_client_dto = ClientDTO {
                    client_position: self.client_position,
                    client_id: self.client_id.clone(),
                    client_order: None, // No hay orden activa
                    time_stamp: std::time::SystemTime::now(),
                };
                self.send_network_message(NetworkMessage::RequestNearbyRestaurants(
                    RequestNearbyRestaurants {
                        client: new_client_dto,
                    },
                ));
            }

            // Client messages
            NetworkMessage::NearbyRestaurants(msg_data) => {
                self.logger.info(format!(
                    "Received NearbyRestaurants message with {} restaurants",
                    msg_data.restaurants.len()
                ));
                ctx.address().do_send(msg_data);
            }
            NetworkMessage::AuthorizationResult(_msg_data) => {
                self.logger
                    .info("Received AuthorizationResult message, not implemented yet");
            }
            NetworkMessage::NotifyOrderUpdated(msg_data) => {
                self.logger.info(format!(
                    "Your order is now: {:?}",
                    msg_data.order.status.to_string().to_uppercase()
                ));
                self.client_order = Some(msg_data.order.clone());
            }
            NetworkMessage::DeliveryExpectedTime(msg_data) => {
                if msg_data.order.client_id == self.client_id {
                    self.logger.info(format!(
                        "Expected delivery time for my order: {} seconds",
                        msg_data.expected_time
                    ));

                    // Se quede esperando el tiempo de entrega
                    let delivery_time = msg_data.expected_time;
                    ctx.run_later(
                        std::time::Duration::from_secs(delivery_time),
                        move |act, _ctx| {
                            act.logger.info(format!(
                                "Delivery expected time of {} seconds has elapsed.",
                                delivery_time
                            ));
                            // Chequeamos si la orden que tengo tien el estado de delivered
                            if let Some(order) = &act.client_order {
                                if order.status == OrderStatus::Delivered {
                                    act.logger.info(format!(
                                        "Order {} has been delivered successfully.",
                                        order.order_id
                                    ));
                                } else {
                                    act.logger.warn(format!(
                                        "Expected delivery time elapsed, sending order has been delivered",
                                    ));
                                    let mut order = order.clone();
                                    order.status = OrderStatus::Delivered;
                                    act.client_order = Some(order.clone());
                                    act.send_network_message(NetworkMessage::OrderDelivered( OrderDelivered {
                                        order: order.clone(),
                                    }));
                                }
                            } else {
                                act.logger.warn("No active order found for delivery check.");
                            }
                        },
                    );
                } else {
                    self.logger.warn(format!(
                        "Received delivery expected time for a different client ({}), ignoring",
                        msg_data.order.client_id
                    ));
                }
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
