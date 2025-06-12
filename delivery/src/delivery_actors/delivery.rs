use actix::prelude::*;
use common::constants::BASE_DELAY_MILLIS;
use common::logger::Logger;
use common::messages::delivery_messages::{AcceptOrder, IAmAvailable, OrderDelivered};
use common::messages::restaurant_messages::DeliverThisOrder;
use common::messages::{DeliveryNoNeeded, NewOfferToDeliver, shared_messages::*};
use common::network::communicator::Communicator;
use common::network::connections::{connect, connect_to_all};
use common::network::peer_types::PeerType;
use common::types::delivery_status::DeliveryStatus;
use common::types::dtos::{DeliveryDTO, OrderDTO, UserDTO};
use common::utils::calculate_distance;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::sleep;

pub struct Delivery {
    /// Vector de direcciones de servidores
    pub servers: Vec<SocketAddr>,
    /// Identificador único del delivery.
    pub delivery_id: String,
    /// Posición actual del delivery.
    pub position: (f32, f32),
    /// Estado actual del delivery: Disponible, Ocupado, Entregando.
    pub status: DeliveryStatus,
    //  Probabilidad de que rechace un pedido disponible de un restaurante.
    pub probability: f32,
    /// Pedido actual en curso, si lo hay.
    pub current_order: Option<OrderDTO>,
    /// Comunicador asociado al Server.
    pub actual_communicator_addr: Option<SocketAddr>,
    pub communicators: Option<HashMap<SocketAddr, Communicator<Delivery>>>,
    pub pending_streams: HashMap<SocketAddr, TcpStream>, // Guarda los streams hasta que arranque
    pub my_socket_addr: SocketAddr,
    pub logger: Logger,
}

impl Delivery {
    pub async fn new(
        servers: Vec<SocketAddr>,
        id: String,
        position: (f32, f32),
        probability: f32,
    ) -> Self {
        let pending_streams: HashMap<SocketAddr, TcpStream> = connect_to_all(servers.clone()).await;

        let logger = Logger::new(format!("Delivery {}", &id));

        if pending_streams.is_empty() {
            panic!("Unable to connect to any server.");
        }

        let my_socket_addr = pending_streams
            .values()
            .next()
            .expect("No stream available")
            .local_addr()
            .expect("Failed to get local socket addr");

        Self {
            servers,
            delivery_id: id,
            position,
            status: DeliveryStatus::Initial,
            probability,
            current_order: None,
            actual_communicator_addr: None,
            communicators: Some(HashMap::new()),
            pending_streams: pending_streams,
            my_socket_addr,
            logger,
        }
    }

    pub fn send_network_message(&self, message: NetworkMessage) {
        if let (Some(communicators_map), Some(addr)) =
            (&self.communicators, self.actual_communicator_addr)
        {
            if let Some(communicator) = communicators_map.get(&addr) {
                if let Some(sender) = communicator.sender.as_ref() {
                    sender.do_send(message);
                } else {
                    self.logger.error("Sender not initialized in communicator");
                }
            } else {
                self.logger
                    .error(&format!("Communicator not found for addr {}", addr));
            }
        } else {
            self.logger
                .error("Communicators map or actual_communicator_addr not initialized");
        }
    }
}

impl Actor for Delivery {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let mut communicators_map = HashMap::new();

        for (addr, stream) in self.pending_streams.drain() {
            let communicator = Communicator::new(stream, ctx.address(), PeerType::DeliveryType);
            communicators_map.insert(addr, communicator);
            self.actual_communicator_addr = Some(addr);
            self.logger
                .info(format!("Communicator started for {}", addr));
        }
        self.communicators = Some(communicators_map);
    }
}

impl Handler<StartRunning> for Delivery {
    type Result = ();

    fn handle(&mut self, _msg: StartRunning, _ctx: &mut Self::Context) {
        self.logger.info("Starting delivery...");
        self.status = DeliveryStatus::Reconnecting;
        self.send_network_message(NetworkMessage::WhoIsLeader(WhoIsLeader {
            origin_addr: self.my_socket_addr,
            user_id: self.delivery_id.clone(),
        }));
    }
}

impl Handler<NewLeaderConnection> for Delivery {
    type Result = ();

    fn handle(&mut self, msg: NewLeaderConnection, ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Creating new Communicator for leader {}", msg.addr));
        let communicator = Communicator::new(msg.stream, ctx.address(), PeerType::DeliveryType);
        if let Some(communicators_map) = &mut self.communicators {
            communicators_map.insert(msg.addr, communicator);
            self.actual_communicator_addr = Some(msg.addr);
            self.logger.info(format!(
                "New leader Communicator established at {}",
                msg.addr
            ));
        } else {
            self.logger
                .error("Communicators map not initialized when creating new leader Communicator");
        }
    }
}

impl Handler<LeaderIs> for Delivery {
    type Result = ();

    fn handle(&mut self, msg: LeaderIs, ctx: &mut Self::Context) -> Self::Result {
        let leader_addr = msg.coord_addr;

        self.logger.info(format!(
            "Received LeaderIs message with addr: {}",
            leader_addr
        ));

        // Verificar si ya tenemos un Communicator para el nuevo líder
        if let Some(communicators_map) = &mut self.communicators {
            if communicators_map.contains_key(&leader_addr) {
                // Ya tenemos conexión con el líder, actualizar
                self.actual_communicator_addr = Some(leader_addr);
                self.logger.info(format!(
                    "Updated actual_communicator_addr to {}",
                    leader_addr
                ));
            } else {
                // No existe aún, tenemos que crearla (async dentro de sync handler usando spawn)
                let delivery_addr = ctx.address();
                let logger_clone = self.logger.clone();
                actix::spawn(async move {
                    logger_clone.info(format!("Connecting to new leader at {}", leader_addr));
                    if let Some(stream) = connect(leader_addr).await {
                        logger_clone.info(format!(
                            "Successfully connected to leader at {}",
                            leader_addr
                        ));

                        delivery_addr.do_send(NewLeaderConnection {
                            addr: leader_addr,
                            stream,
                        });
                    } else {
                        logger_clone.error(format!(
                            "Failed to connect to new leader at {}",
                            leader_addr
                        ));
                    }
                });
            }
            self.send_network_message(NetworkMessage::RegisterUser(RegisterUser {
                origin_addr: self.my_socket_addr,
                user_id: self.delivery_id.clone(),
            }));
            self.logger.info(format!(
                "Sending msg register user with ID: {}",
                self.delivery_id
            ));
        } else {
            self.logger.error("Communicators map not initialized");
        }
    }
}

impl Handler<RecoverProcedure> for Delivery {
    type Result = ();

    fn handle(&mut self, msg: RecoverProcedure, ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Handling RecoverProcedure for Delivery ID={}",
            self.delivery_id
        ));
        let delivery_dto = match msg.user_info {
            UserDTO::Delivery(delivery_dto) => delivery_dto,
            _ => {
                self.logger
                    .error("RecoverProcedure: user_info no es DeliveryDTO");
                return;
            }
        };
        let order_dto = delivery_dto.current_order.clone();

        // Actualizar el estado del delivery con la información recuperada
        self.position = delivery_dto.delivery_position;
        self.status = delivery_dto.status;
        self.current_order = order_dto;

        self.logger.info(format!(
            "Updated position=({:?}), status={:?}, current_order_id={:?}",
            self.position,
            self.status,
            self.current_order.as_ref().map(|o| o.order_id),
        ));

        self.logger.warn("Handling RecoverProcedure not fully implemented, please implement the necessary logic based on the delivery state.");
        match self.status {
            DeliveryStatus::Initial => {
                self.logger
                    .info("Delivery is in Initial state, sending StartRunning");
                ctx.address().do_send(StartRunning);
            }
            DeliveryStatus::Reconnecting => {
                self.logger
                    .info("Delivery is in Reconnecting state, sending WhoIsLeader");
                self.send_network_message(NetworkMessage::WhoIsLeader(WhoIsLeader {
                    origin_addr: self.my_socket_addr,
                    user_id: self.delivery_id.clone(),
                }));
            }
            /*
            DeliveryStatus::Recovering => {
                self.logger.info("Delivery is in Recovering state, sending RecoverProcedure");
                ctx.address().do_send(RecoverProcedure { delivery_info: delivery_dto.clone() });
            }
            */
            DeliveryStatus::Available => {
                self.logger
                    .info("Delivery is Available, sending IAmAvailable");
                self.send_network_message(NetworkMessage::IAmAvailable(IAmAvailable {
                    delivery_info: delivery_dto.clone(),
                }));
            }
            DeliveryStatus::WaitingConfirmation => {
                if let Some(order) = &self.current_order {
                    self.logger.info(format!(
                        "Delivery is WaitingConfirmation for order {}",
                        order.order_id
                    ));
                    self.send_network_message(NetworkMessage::AcceptOrder(AcceptOrder {
                        order: order.clone(),
                    }));
                } else {
                    self.logger
                        .warn("No current order available while in WaitingConfirmation state.");
                    self.status = DeliveryStatus::Available;
                    self.send_network_message(NetworkMessage::IAmAvailable(IAmAvailable {
                        delivery_info: delivery_dto.clone(),
                    }));
                }
            }
            DeliveryStatus::Delivering => {
                if let Some(order) = &self.current_order {
                    self.logger
                        .info(format!("Delivery is Delivering order {}", order.order_id));
                    if order.client_position == self.position {
                        self.logger.info(
                            "Delivery position matches order location, sending OrderDelivered",
                        );
                        self.status = DeliveryStatus::Available;
                        ctx.address()
                            .do_send(NetworkMessage::OrderDelivered(OrderDelivered {
                                order: order.clone(),
                            }));
                    } else {
                        self.logger.warn(
                            "Delivery position does not match order location, sending OrderUpdate",
                        );
                        // Here you would send an update about the delivery status
                    }
                } else {
                    self.logger
                        .warn("No current order available while in Delivering state.");
                    self.status = DeliveryStatus::Available;
                    self.send_network_message(NetworkMessage::IAmAvailable(IAmAvailable {
                        delivery_info: delivery_dto.clone(),
                    }));
                }
            }
            _ => {
                self.logger
                    .error(format!("Unknown DeliveryStatus: {:?}", self.status));
            }
        }
        self.logger.info("RecoverProcedure handling completed.");
    }
}

impl Handler<NewOfferToDeliver> for Delivery {
    type Result = ();

    fn handle(&mut self, msg: NewOfferToDeliver, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Received NewOfferToDeliver for order ID: {}",
            msg.order.order_id
        ));
        match self.status {
            // Si estoy disponible, acepto el pedido
            DeliveryStatus::Available => {
                self.logger
                    .info(format!("Accepting order ID: {}", msg.order.order_id));
                // Probabilidad de aceptar el pedido
                let accept_order = rand::random::<f32>() < self.probability;
                if !accept_order {
                    self.logger.warn(format!(
                        "Order ID: {} rejected by delivery with probability {}",
                        msg.order.order_id, self.probability
                    ));
                    return;
                }
                self.current_order = Some(msg.order.clone());
                self.status = DeliveryStatus::WaitingConfirmation;
                self.send_network_message(NetworkMessage::AcceptOrder(AcceptOrder {
                    order: msg.order,
                }));
            }
            // Si estoy ocupado, ignoro el pedido
            DeliveryStatus::WaitingConfirmation => {
                self.logger
                    .warn("Already waiting for confirmation, ignoring new offer.");
            }
            _ => {
                self.logger.warn(format!(
                    "Delivery is not available to accept new offers, current status: {:?}",
                    self.status
                ));
            }
        }
    }
}

impl Handler<DeliveryNoNeeded> for Delivery {
    type Result = ();

    fn handle(&mut self, msg: DeliveryNoNeeded, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(current_order) = &self.current_order {
            if current_order.order_id == msg.order.order_id {
                self.logger
                    .info("Order no longer needed, resetting current order.");
                self.current_order = None;
                self.status = DeliveryStatus::Available;
            } else {
                self.logger.warn(format!(
                    "Received DeliveryNoNeeded for a different order ({}), ignoring",
                    msg.order.order_id
                ));
            }
        } else {
            self.logger.warn("No current order to cancel.");
        }
    }
}

impl Handler<DeliverThisOrder> for Delivery {
    type Result = ();

    fn handle(&mut self, msg: DeliverThisOrder, ctx: &mut Self::Context) -> Self::Result {
        if let Some(current_order) = &self.current_order {
            if current_order.order_id == msg.order.order_id {
                self.logger.info(format!(
                    "Delivering order ID: {} to position: {:?}",
                    msg.order.order_id, msg.order.client_position
                ));
                self.status = DeliveryStatus::Delivering;

                // Simular el tiempo de entrega basado en la distancia
                let distance = calculate_distance(self.position, msg.order.client_position);
                let delay_ms = BASE_DELAY_MILLIS + (distance as u64 * 1000);
                self.position = msg.order.client_position;
                let addr = ctx.address().clone();
                let order = msg.order.clone();
                actix::spawn(async move {
                    sleep(Duration::from_millis(delay_ms)).await;
                    addr.do_send(OrderDelivered {
                        order: order.clone(),
                    });
                });

                self.position = msg.order.client_position;
            } else {
                self.logger.warn(format!(
                    "Received DeliverThisOrder for a different order ({}), ignoring",
                    msg.order.order_id
                ));
            }
        } else {
            self.logger.warn("No current order to deliver.");
        }
    }
}

impl Handler<OrderDelivered> for Delivery {
    type Result = ();

    fn handle(&mut self, msg: OrderDelivered, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(current_order) = &self.current_order {
            if current_order.order_id == msg.order.order_id {
                self.logger.info(format!(
                    "Order ID: {} delivered successfully.",
                    msg.order.order_id
                ));
                self.current_order = None;
                self.status = DeliveryStatus::Available;
                let my_delivery_info = DeliveryDTO {
                    delivery_id: self.delivery_id.clone(),
                    delivery_position: self.position,
                    status: self.status.clone(),
                    current_order: None, // No current order after delivery
                    current_client_id: None,
                    time_stamp: std::time::SystemTime::now(),
                };

                self.send_network_message(NetworkMessage::IAmAvailable(IAmAvailable {
                    delivery_info: my_delivery_info.clone(),
                }));
            } else {
                self.logger.warn(format!(
                    "Received OrderDelivered for a different order ({}), ignoring",
                    msg.order.order_id
                ));
            }
        } else {
            self.logger.warn("No current order to mark as delivered.");
        }
    }
}

impl Handler<NetworkMessage> for Delivery {
    type Result = ();
    fn handle(&mut self, msg: NetworkMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            // Users messages
            NetworkMessage::LeaderIs(msg_data) => ctx.address().do_send(msg_data),
            NetworkMessage::RecoveredInfo(user_dto_opt) => match user_dto_opt {
                user_dto => match user_dto {
                    UserDTO::Delivery(delivery_dto) => {
                        if delivery_dto.delivery_id == self.delivery_id {
                            self.logger.info(format!(
                                "Recovered info for Delivery ID={}, updating local state...",
                                delivery_dto.delivery_id
                            ));
                            ctx.address().do_send(RecoverProcedure {
                                user_info: UserDTO::Delivery(delivery_dto.clone()),
                            });
                        } else {
                            self.logger.warn(format!(
                                "Received recovered info for a different delivery ({}), ignoring",
                                delivery_dto.delivery_id
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
            // TODO: Handle NoRecoveredInfo if needed
            NetworkMessage::NewOfferToDeliver(msg_data) => {
                self.logger.info(format!(
                    "Received NewOfferToDeliver for order ID: {}",
                    msg_data.order.order_id
                ));
                ctx.address().do_send(msg_data);
            }
            NetworkMessage::DeliveryNoNeeded(msg_data) => {
                self.logger.info(format!(
                    "Received DeliveryNoNeeded for order ID: {}",
                    msg_data.order.order_id
                ));
                ctx.address().do_send(msg_data);
            }
            NetworkMessage::DeliverThisOrder(msg_data) => {
                self.logger.info(format!(
                    "Received DeliverThisOrder for order ID: {}",
                    msg_data.order.order_id
                ));
                ctx.address().do_send(msg_data);
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

// ------- TESTING ------- //
struct GetStatus;
impl Message for GetStatus {
    type Result = UserDTO;
}
impl Handler<GetStatus> for Delivery {
    type Result = MessageResult<GetStatus>;
    fn handle(&mut self, _msg: GetStatus, _ctx: &mut Self::Context) -> Self::Result {
        MessageResult(UserDTO::Delivery(DeliveryDTO {
            delivery_id: self.delivery_id.clone(),
            delivery_position: self.position,
            status: self.status.clone(),
            current_order: self.current_order.clone(),
            current_client_id: self.current_order.as_ref().map(|o| o.client_id.clone()),
            time_stamp: std::time::SystemTime::now(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::types::order_status::OrderStatus;
    use std::collections::HashMap;

    fn test_order(order_id: u64) -> OrderDTO {
        OrderDTO {
            order_id,
            dish_name: "Pizza".to_string(),
            client_id: "client1".to_string(),
            restaurant_id: "rest1".to_string(),
            delivery_id: Some("delivery1".to_string()),
            status: OrderStatus::Pending,
            client_position: (1.0, 2.0),
            time_stamp: std::time::SystemTime::now(),
        }
    }

    fn dummy_delivery(status: DeliveryStatus, current_order: Option<OrderDTO>) -> Delivery {
        Delivery {
            servers: vec![],
            delivery_id: "delivery1".to_string(),
            position: (0.0, 0.0),
            status,
            probability: 1.0,
            current_order,
            actual_communicator_addr: None,
            communicators: Some(HashMap::new()),
            pending_streams: HashMap::new(),
            my_socket_addr: "127.0.0.1:12345".parse().unwrap(),
            logger: Logger::new("TestDelivery".to_string()),
        }
    }

    #[actix_rt::test]
    async fn test_handle_new_offer_to_deliver_accepts_order() {
        let delivery = dummy_delivery(DeliveryStatus::Available, None).start();
        let order = test_order(1);
        let msg = NewOfferToDeliver {
            order: order.clone(),
        };

        // Enviamos el mensaje y esperamos a que se procese
        delivery.send(msg).await.unwrap();

        // Consultar el estado del actor Delivery
        let result = delivery.send(GetStatus).await.unwrap();
        if let UserDTO::Delivery(delivery_dto) = result {
            assert_eq!(delivery_dto.status, DeliveryStatus::WaitingConfirmation);
            assert_eq!(delivery_dto.current_order.unwrap().order_id, 1);
        } else {
            panic!("Expected UserDTO::Delivery variant");
        }
    }

    #[actix_rt::test]
    async fn test_handle_delivery_no_needed_resets_order() {
        let order = test_order(2);
        let delivery =
            dummy_delivery(DeliveryStatus::WaitingConfirmation, Some(order.clone())).start();

        // Primero simulamos que el delivery tiene un pedido pendiente
        // Enviamos el mensaje DeliveryNoNeeded
        delivery
            .send(DeliveryNoNeeded {
                order: order.clone(),
            })
            .await
            .unwrap();

        // Consultamos el estado
        let result = delivery.send(GetStatus).await.unwrap();
        if let UserDTO::Delivery(delivery_dto) = result {
            assert_eq!(delivery_dto.status, DeliveryStatus::Available);
            assert!(delivery_dto.current_order.is_none());
        } else {
            panic!("Expected UserDTO::Delivery variant");
        }
    }

    #[actix_rt::test]
    async fn test_handle_deliver_this_order_sets_delivering() {
        let order = test_order(3);
        let delivery =
            dummy_delivery(DeliveryStatus::WaitingConfirmation, Some(order.clone())).start();

        // Enviamos el mensaje DeliverThisOrder
        delivery
            .send(DeliverThisOrder {
                order: order.clone(),
            })
            .await
            .unwrap();

        // Consultamos el estado
        let result = delivery.send(GetStatus).await.unwrap();
        if let UserDTO::Delivery(delivery_dto) = result {
            assert_eq!(delivery_dto.status, DeliveryStatus::Delivering);
            assert_eq!(delivery_dto.delivery_position, order.client_position);
        } else {
            panic!("Expected UserDTO::Delivery variant");
        }
    }

    #[actix_rt::test]
    async fn test_handle_order_delivered_sets_available() {
        let order = test_order(4);
        let delivery = dummy_delivery(DeliveryStatus::Delivering, Some(order.clone())).start();

        // Enviamos el mensaje OrderDelivered
        delivery
            .send(OrderDelivered {
                order: order.clone(),
            })
            .await
            .unwrap();

        // Consultamos el estado
        let result = delivery.send(GetStatus).await.unwrap();
        if let UserDTO::Delivery(delivery_dto) = result {
            assert_eq!(delivery_dto.status, DeliveryStatus::Available);
            assert!(delivery_dto.current_order.is_none());
        } else {
            panic!("Expected UserDTO::Delivery variant");
        }
    }

    #[actix_rt::test]
    async fn test_network_message_dispatch() {
        let order = test_order(5);
        let delivery = dummy_delivery(DeliveryStatus::Available, None).start();

        // NewOfferToDeliver
        delivery
            .send(NetworkMessage::NewOfferToDeliver(NewOfferToDeliver {
                order: order.clone(),
            }))
            .await
            .unwrap();
        let result = delivery.send(GetStatus).await.unwrap();
        if let UserDTO::Delivery(delivery_dto) = result {
            assert_eq!(delivery_dto.status, DeliveryStatus::WaitingConfirmation);
        } else {
            panic!("Expected UserDTO::Delivery variant");
        }

        // DeliveryNoNeeded
        delivery
            .send(NetworkMessage::DeliveryNoNeeded(DeliveryNoNeeded {
                order: order.clone(),
            }))
            .await
            .unwrap();
        let result = delivery.send(GetStatus).await.unwrap();
        if let UserDTO::Delivery(delivery_dto) = result {
            assert_eq!(delivery_dto.status, DeliveryStatus::Available);
        } else {
            panic!("Expected UserDTO::Delivery variant");
        }

        // DeliverThisOrder
        delivery
            .send(NetworkMessage::NewOfferToDeliver(NewOfferToDeliver {
                order: order.clone(),
            }))
            .await
            .unwrap();
        delivery
            .send(NetworkMessage::DeliverThisOrder(DeliverThisOrder {
                order: order.clone(),
            }))
            .await
            .unwrap();
        let result = delivery.send(GetStatus).await.unwrap();
        if let UserDTO::Delivery(delivery_dto) = result {
            assert_eq!(delivery_dto.status, DeliveryStatus::Delivering);
        } else {
            panic!("Expected UserDTO::Delivery variant");
        }
    }
}
