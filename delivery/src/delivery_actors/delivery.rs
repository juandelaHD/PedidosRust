use actix::fut::wrap_future;
use actix::prelude::*;
use common::constants::BASE_DELAY_MILLIS;
use common::logger::Logger;
use common::messages::delivery_messages::{AcceptOrder, IAmAvailable, OrderDelivered};
use common::messages::{shared_messages::*, DeliverThisOrder, DeliveryNoNeeded, NewOfferToDeliver};
use common::network::communicator::Communicator;
use common::network::connections::{connect_some, try_to_connect};
use common::network::peer_types::PeerType;
use common::types::delivery_status::DeliveryStatus;
use common::types::dtos::{DeliveryDTO, OrderDTO, UserDTO};
use common::utils::calculate_distance;
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
    pub communicator: Option<Communicator<Delivery>>,
    pub pending_stream: Option<TcpStream>, // Guarda los streams hasta que arranque
    pub logger: Logger,
}

impl Delivery {
    pub async fn new(
        servers: Vec<SocketAddr>,
        delivery_id: String,
        position: (f32, f32),
        probability: f32,
    ) -> Self {
        let logger = Logger::new(format!("Delivery {}", &delivery_id));
        logger.info(format!("Starting delivery with ID: {}", delivery_id));
        // Intentamos conectarnos a los servidores
        let pending_stream = connect_some(servers.clone(), PeerType::DeliveryType).await;

        if pending_stream.is_none() {
            panic!("Failed to connect to any server. Try again later.");
        }

        Self {
            servers,
            delivery_id,
            position,
            status: DeliveryStatus::Available,
            probability,
            current_order: None,
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
            self.logger
                .error("Communicator not initialized, cannot send network message");
        }
    }

    pub fn start_running(&self, _ctx: &mut Context<Self>) {
        self.logger.info("Starting delivery...");
        let actual_socket_addr = self
            .communicator
            .as_ref()
            .map(|c| c.local_address)
            .expect("Socket address not initialized");
        self.send_network_message(NetworkMessage::WhoIsLeader(WhoIsLeader {
            origin_addr: actual_socket_addr,
            user_id: self.delivery_id.clone(),
        }));
    }
}

impl Actor for Delivery {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let communicator = Communicator::new(
            self.pending_stream
                .take()
                .expect("Pending stream should be set"),
            ctx.address(),
            PeerType::DeliveryType,
        );
        self.communicator = Some(communicator);
        self.start_running(ctx);
    }
}

pub struct UpdateCommunicator(pub Communicator<Delivery>);

impl Message for UpdateCommunicator {
    type Result = ();
}

impl Handler<UpdateCommunicator> for Delivery {
    type Result = ();

    fn handle(&mut self, msg: UpdateCommunicator, _ctx: &mut Self::Context) -> Self::Result {
        self.communicator = Some(msg.0);
    }
}

pub struct SendRegistration();

impl Message for SendRegistration {
    type Result = ();
}

impl Handler<SendRegistration> for Delivery {
    type Result = ();

    fn handle(&mut self, _msg: SendRegistration, _ctx: &mut Self::Context) -> Self::Result {
        let local_address = self
            .communicator
            .as_ref()
            .map(|c| c.local_address)
            .expect("Socket address not set");
        self.send_network_message(NetworkMessage::RegisterUser(RegisterUser {
            origin_addr: local_address,
            user_id: self.delivery_id.clone(),
            position: self.position.clone(),
        }));
    }
}

impl Handler<LeaderIs> for Delivery {
    type Result = ();

    fn handle(&mut self, msg: LeaderIs, ctx: &mut Self::Context) -> Self::Result {
        let leader_addr = msg.coord_addr;
        let self_addr = ctx.address();
        let logger = self.logger.clone();
        let communicator_opt = self.communicator.as_ref().map(|c| c.peer_address);

        // Si ya estamos conectados al líder, enviamos que nos registre
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
                user_id: self.delivery_id.clone(),
                position: self.position,
            }));
            return;
        }
        // Si no estamos conectados al líder, intentamos conectarnos
        // Clone the necessary fields to move into the async block
        ctx.spawn(wrap_future(async move {
            if let Some(new_stream) = try_to_connect(leader_addr).await {
                let new_communicator =
                    Communicator::new(new_stream, self_addr.clone(), PeerType::DeliveryType);
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
            user_id: self.delivery_id.clone(),
        }));
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

        match self.status {
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
                    .info("Delivery is available or in another state, no action needed.");
            }
        }
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
                    status: self.status,
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

            NetworkMessage::NoRecoveredInfo => {
                self.logger
                    .warn("No recovered info available, proceeding with normal operation.");
            }

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

/*
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
*/
