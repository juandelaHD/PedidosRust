use actix::fut::wrap_future;
use actix::prelude::*;
use colored::Color;
use common::constants::{BASE_DELAY_MILLIS, DELAY_SECONDS_TO_START_RECONNECT};
use common::logger::Logger;
use common::messages::delivery_messages::*;
use common::messages::shared_messages::*;
use common::messages::{
    AcceptedOrder, DeliverThisOrder, DeliveryNoNeeded, LeaderIs, NetworkMessage, NewOfferToDeliver,
    RecoverProcedure, UpdateOrderStatus, WhoIsLeader,
};

use common::network::communicator::Communicator;
use common::network::connections::{connect_one, connect_some, reconnect};
use common::network::peer_types::PeerType;
use common::types::delivery_status::DeliveryStatus;
use common::types::dtos::{DeliveryDTO, OrderDTO, UserDTO};
use common::types::order_status::OrderStatus;
use common::utils::calculate_distance;
use std::net::SocketAddr;
use std::process;
use std::time::Duration;
use tokio::net::TcpStream;

/// The `Delivery` actor represents a delivery person in the distributed restaurant ordering system.
///
/// This actor is responsible for:
/// - Registering itself with the server cluster.
/// - Receiving and accepting delivery offers.
/// - Simulating the delivery process (including travel and delivery time).
/// - Updating its status and reporting order delivery.
/// - Handling recovery and reconnection scenarios.
pub struct Delivery {
    /// List of server socket addresses to connect to.
    pub servers: Vec<SocketAddr>,
    /// Unique identifier for the delivery actor.
    pub delivery_id: String,
    /// Current position of the delivery actor.
    pub position: (f32, f32),
    /// Current status of the delivery actor (Available, Busy, Delivering, etc.).
    pub status: DeliveryStatus,
    /// Probability of rejecting an available order.
    pub probability: f32,
    /// Current order being delivered, if any.
    pub current_order: Option<OrderDTO>,
    /// Communicator for network interactions with the server.
    pub communicator: Option<Communicator<Delivery>>,
    /// Pending TCP stream before the actor starts.
    pub pending_stream: Option<TcpStream>,
    /// Logger for delivery events.
    pub logger: Logger,
    /// Timer handle for waiting reconnection attempts.
    waiting_reconnection_timer: Option<actix::SpawnHandle>,
    /// Timer handle for keeping the actor alive during reconnection attempts.
    keep_alive_timer: Option<actix::SpawnHandle>,
    /// Flag to indicate if the delivery is already connected and waiting for reconnection.
    already_connected: bool,
}

impl Delivery {
    /// Creates a new `Delivery` instance and attempts to connect to the server cluster.
    ///
    /// # Arguments
    ///
    /// * `servers` - A vector of server socket addresses.
    /// * `delivery_id` - The unique identifier for the delivery actor.
    /// * `position` - The initial position of the delivery actor.
    /// * `probability` - Probability of rejecting an order.
    ///
    /// # Returns
    ///
    /// Returns a new `Delivery` instance.
    pub async fn new(
        servers: Vec<SocketAddr>,
        delivery_id: String,
        position: (f32, f32),
        probability: f32,
    ) -> Self {
        let logger = Logger::new(format!("Delivery {}", &delivery_id), Color::BrightGreen);
        logger.info(format!("Hello: {}!", delivery_id));
        // Intentamos conectarnos a los servidores
        let pending_stream = connect_some(servers.clone(), PeerType::DeliveryType).await;

        if pending_stream.is_none() {
            logger.error(format!(
                "Failed to connect to any server from the list: {:?}",
                servers
            ));
            std::process::exit(1);
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
            waiting_reconnection_timer: None,
            keep_alive_timer: None,
            already_connected: false,
        }
    }

    /// Sends a network message to the connected server via the communicator.
    ///
    /// # Arguments
    ///
    /// * `message` - The network message to send.
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

    /// Starts the delivery logic by requesting the current leader from the server.
    ///
    /// # Arguments
    ///
    /// * `_ctx` - The Actix actor context.
    pub fn start_running(&self, _ctx: &mut Context<Self>) {
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

    /// Calculates the delivery delay in milliseconds based on the distance from the delivery's
    /// current position to the restaurant and from the restaurant to the client.
    ///
    /// # Arguments
    ///
    /// * `restaurant_position` - The position of the restaurant.
    /// * `client_position` - The position of the client.
    /// * `base_delay_millis` - The base delay in milliseconds.
    ///
    /// # Returns
    ///
    /// Returns the total estimated delivery delay in milliseconds.
    pub fn calcular_delay_ms(
        &self,
        restaurant_position: (f32, f32),
        client_position: (f32, f32),
        base_delay_millis: u64,
    ) -> u64 {
        let distance_from_restaurant = calculate_distance(self.position, restaurant_position);
        let distance_restaurant_from_client =
            calculate_distance(restaurant_position, client_position);
        let total_distance = distance_from_restaurant + distance_restaurant_from_client;
        base_delay_millis + (total_distance as u64 * 1000)
    }
}

/// Handles [`ConnectionClosed`] messages.
///
/// This handler is triggered when the connection to the server is lost.
/// It attempts to reconnect to one of the known servers. If reconnection is successful,
/// it reinitializes the communicator and restarts the actor. If not, the actor is stopped.
impl Handler<ConnectionClosed> for Delivery {
    type Result = ();

    fn handle(&mut self, _msg: ConnectionClosed, ctx: &mut Self::Context) -> Self::Result {
        println!("[Delivery][ConnectionClosed] Handler llamado");
        println!(
            "[Delivery][ConnectionClosed] Estado communicator: {:?}",
            self.communicator.is_some()
        );
        let servers = self.servers.clone();
        println!(
            "[Delivery][ConnectionClosed] Llamando a reconnect con: {:?}",
            servers
        );
        let fut = async move { reconnect(servers, PeerType::DeliveryType).await };

        let fut = wrap_future::<_, Self>(fut).map(|result, actor: &mut Self, ctx| {
            println!("[Delivery][ConnectionClosed] Futuro de reconexión terminado");
            match result {
                Some(stream) => {
                    println!(
                        "[Delivery][ConnectionClosed] Reconexión exitosa, creando communicator"
                    );
                    let communicator =
                        Communicator::new(stream, ctx.address(), PeerType::DeliveryType);
                    actor.communicator = Some(communicator);

                    actor
                        .logger
                        .info("Reconnected successfully. Restarting actor...");

                    if let Some(handler) = actor.waiting_reconnection_timer.take() {
                        ctx.cancel_future(handler);
                        actor.waiting_reconnection_timer = None;
                        println!("[Delivery][ConnectionClosed] CANCELANDO TIMER DE RECONEXIÓN");
                    }

                    let addr = ctx.address();
                    let handler =
                        ctx.run_later(std::time::Duration::from_millis(100), move |_, _| {
                            println!(
                                "[Delivery][ConnectionClosed] Enviando StartRunning tras reconexión"
                            );
                            addr.do_send(StartRunning);
                        });
                    actor.waiting_reconnection_timer = Some(handler);
                }
                None => {
                    println!(
                        "[Delivery][ConnectionClosed] No se pudo reconectar, deteniendo actor"
                    );
                    actor
                        .logger
                        .error("Failed to reconnect to any server after closed connection");
                    ctx.stop();
                }
            }
        });
        ctx.spawn(fut);
    }
}

impl Actor for Delivery {
    type Context = Context<Self>;

    /// Called when the `Delivery` actor is started.
    ///
    /// Initializes the network communicator and begins the delivery logic.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The Actix actor context.
    fn started(&mut self, ctx: &mut Self::Context) {
        let communicator = Communicator::new(
            self.pending_stream
                .take()
                .expect("Pending stream should be set"),
            ctx.address(),
            PeerType::DeliveryType,
        );
        self.communicator = Some(communicator);
        // Esperar 100ms antes de enviar WhoIsLeader
        let addr = ctx.address();
        let handler = ctx.run_later(std::time::Duration::from_millis(100), move |_, _| {
            addr.do_send(StartRunning);
        });
        self.waiting_reconnection_timer = Some(handler);
    }
}

impl Handler<LeaderIs> for Delivery {
    type Result = ();

    fn handle(&mut self, msg: LeaderIs, ctx: &mut Self::Context) -> Self::Result {
        println!(
            "[Delivery][LeaderIs] Recibido LeaderIs: {:?}",
            msg.coord_addr
        );
        let leader_addr = msg.coord_addr;

        let communicator_opt = self.communicator.as_ref().map(|c| c.peer_address);

        println!(
            "[Delivery][LeaderIs] Comparando leader_addr: {:?} con communicator.peer_address: {:?}",
            leader_addr, communicator_opt
        );

        if Some(leader_addr) == communicator_opt {
            println!("[Delivery][LeaderIs] Ya conectado al líder");
            // CANCELA EL KEEP-ALIVE SI EXISTE
            if let Some(handler) = self.keep_alive_timer.take() {
                ctx.cancel_future(handler);
                println!("[Delivery][LeaderIs] KEEP-ALIVE CANCELADO (ya conectado al líder)");
            }

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

        if let Some(comm) = self.communicator.as_mut() {
            comm.shutdown();
        }
        self.communicator = None;

        let fut_connect = async move { connect_one(leader_addr, PeerType::DeliveryType).await };

        let fut = wrap_future::<_, Self>(fut_connect).map(|result, actor: &mut Self, ctx| {
            println!("[Delivery][LeaderIs] Futuro de reconexión terminado");
            match result {
                Some(stream) => {
                    println!("[Delivery][LeaderIs] Reconexión exitosa, creando communicator");
                    let communicator =
                        Communicator::new(stream, ctx.address(), PeerType::DeliveryType);
                    actor.communicator = Some(communicator);

                    actor
                        .logger
                        .info("Reconnected successfully. Restarting actor...");

                    // CANCELA EL KEEP-ALIVE SI EXISTE
                    if let Some(handler) = actor.keep_alive_timer.take() {
                        ctx.cancel_future(handler);
                        println!("[Delivery][LeaderIs] KEEP-ALIVE CANCELADO");
                    }

                    if let Some(handler) = actor.waiting_reconnection_timer.take() {
                        ctx.cancel_future(handler);
                        actor.waiting_reconnection_timer = None;
                        println!("[Delivery][LeaderIs] CANCELANDO TIMER DE RECONEXIÓN");
                    }

                    let addr = ctx.address();
                    let handler =
                        ctx.run_later(std::time::Duration::from_millis(100), move |_, _| {
                            println!("[Delivery][LeaderIs] Enviando StartRunning tras reconexión");
                            addr.do_send(StartRunning);
                        });
                    actor.waiting_reconnection_timer = Some(handler);
                }
                None => {
                    println!(
                        "[Delivery][LeaderIs] No se pudo reconectar al líder, deteniendo actor"
                    );
                    actor
                        .logger
                        .error("Failed to reconnect to any server after closed connection");
                    ctx.stop();
                }
            }
        });

        ctx.spawn(fut);
    }
}
// Mensaje para pedir la dirección local
pub struct GetLocalAddress;

impl Message for GetLocalAddress {
    type Result = Option<SocketAddr>;
}

// Handler para responder con la dirección local
impl Handler<GetLocalAddress> for Delivery {
    type Result = MessageResult<GetLocalAddress>;

    fn handle(&mut self, _msg: GetLocalAddress, _ctx: &mut Self::Context) -> Self::Result {
        let addr = self.communicator.as_ref().map(|c| c.local_address);
        MessageResult(addr)
    }
}

impl Handler<StartRunning> for Delivery {
    type Result = ();

    fn handle(&mut self, _msg: StartRunning, ctx: &mut Self::Context) -> Self::Result {
        self.start_running(ctx);
    }
}

/// Handler for the `RecoverProcedure` message.
///
/// Restores the delivery's state from a recovery message, including position, status, and current order.
/// If in the middle of a delivery, resumes the appropriate process.
impl Handler<RecoverProcedure> for Delivery {
    type Result = ();

    fn handle(&mut self, msg: RecoverProcedure, ctx: &mut Self::Context) -> Self::Result {
        if self.already_connected {
            self.logger
                .info("Already connected, skipping recovery procedure.");
            return;
        }
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
        self.already_connected = true;
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
                    self.send_network_message(NetworkMessage::AcceptedOrder(AcceptedOrder {
                        order: order.clone(),
                        delivery_info: delivery_dto.clone(),
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

                    self.logger
                        .info("Delivery position matches order location, sending OrderDelivered");
                    self.status = DeliveryStatus::Delivering;
                    let distance_from_client =
                        calculate_distance(self.position, order.client_position);
                    let delay_ms = BASE_DELAY_MILLIS + (distance_from_client as u64 * 1000);
                    let mut order = order.clone();
                    order.expected_delivery_time = delay_ms;

                    // Aviso  al servidor que estoy entregando
                    self.send_network_message(NetworkMessage::UpdateOrderStatus(
                        UpdateOrderStatus {
                            order: order.clone(),
                        },
                    ));

                    // Simular el tiempo de entrega
                    let client_position = order.client_position;
                    ctx.run_later(Duration::from_millis(delay_ms), move |_act, ctx| {
                        ctx.address().do_send(OrderDelivered {
                            order: order.clone(),
                        });
                    });
                    self.position = client_position;
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

/// Handler for the `NewOfferToDeliver` message.
///
/// Handles a new delivery offer. If available, may accept the order based on probability.
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
                let my_info = DeliveryDTO {
                    delivery_id: self.delivery_id.clone(),
                    delivery_position: self.position,
                    status: self.status,
                    current_order: Some(msg.order.clone()),
                    current_client_id: Some(msg.order.client_id.clone()),
                    time_stamp: std::time::SystemTime::now(),
                };
                self.send_network_message(NetworkMessage::AcceptedOrder(AcceptedOrder {
                    order: msg.order,
                    delivery_info: my_info.clone(),
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

/// Handler for the `DeliveryNoNeeded` message.
///
/// Handles notification that a delivery is no longer needed, resetting the current order and status.
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

/// Handler for the `DeliverThisOrder` message.
///
/// Simulates the delivery process, updates the order status, and notifies the server upon completion.
impl Handler<DeliverThisOrder> for Delivery {
    type Result = ();

    fn handle(&mut self, msg: DeliverThisOrder, ctx: &mut Self::Context) -> Self::Result {
        if let Some(current_order) = &self.current_order {
            if current_order.order_id == msg.order.order_id {
                self.logger.info(format!(
                    "Delivering order for client '{}' to destination {:?}",
                    current_order.client_id, msg.order.client_position
                ));
                let mut new_order = msg.order.clone();

                // Simular el tiempo de llegada al restaurante y al cliente
                let delay_ms = self.calcular_delay_ms(
                    msg.restaurant_info.position,
                    msg.order.client_position,
                    BASE_DELAY_MILLIS,
                );

                self.logger.info(format!(
                    "Estimated delivery time: {:.2} seconds",
                    delay_ms as f64 / 1000.0
                ));

                new_order.expected_delivery_time = delay_ms;

                self.send_network_message(NetworkMessage::UpdateOrderStatus(UpdateOrderStatus {
                    order: new_order.clone(),
                }));

                let order = msg.order.clone();
                ctx.run_later(Duration::from_millis(delay_ms), move |_act, ctx| {
                    ctx.address().do_send(OrderDelivered {
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

/// Handler for the `OrderDelivered` message.
///
/// Marks the order as delivered, notifies the server, and sets the delivery status to available.
impl Handler<OrderDelivered> for Delivery {
    type Result = ();

    fn handle(&mut self, msg: OrderDelivered, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(current_order) = &self.current_order {
            if current_order.order_id == msg.order.order_id {
                self.logger.info(format!(
                    "Order ID: {} delivered successfully.",
                    msg.order.order_id
                ));
                let mut new_order = msg.order.clone();
                new_order.status = OrderStatus::Delivered;

                self.send_network_message(NetworkMessage::OrderDelivered(OrderDelivered {
                    order: new_order,
                }));

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

/// Handler for all incoming `NetworkMessage` messages.
///
/// Handles various network events, such as leader changes, delivery offers, order updates,
/// and recovery information. Updates the delivery state and interacts with the server as needed.
impl Handler<NetworkMessage> for Delivery {
    type Result = ();
    fn handle(&mut self, msg: NetworkMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            // Users messages
            NetworkMessage::RetryLater(_msg_data) => {
                self.logger.info("Retrying to connect in some seconds");
            }
            NetworkMessage::LeaderIs(msg_data) => ctx.address().do_send(msg_data),
            NetworkMessage::RecoveredInfo(user_dto_opt) => {
                let user_dto = user_dto_opt;
                match user_dto {
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
                }
            }

            NetworkMessage::NoRecoveredInfo => {
                self.logger
                    .warn("No recovered info available, proceeding with normal operation.");
                self.already_connected = true;
            }

            NetworkMessage::NewOfferToDeliver(msg_data) => {
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
            NetworkMessage::ConnectionClosed(msg_data) => {
                println!(
                    "[Delivery][NetworkMessage] ConnectionClosed recibido: {:?}",
                    msg_data.remote_addr
                );
                println!(
                    "[DEBUG] Intentando reconectar a servidores: {:?}",
                    self.servers
                );
                self.logger.info(format!(
                    "Connection closed with address: {}",
                    msg_data.remote_addr
                ));

                self.logger.info(format!(
                    "Removing communicator for address: {}",
                    msg_data.remote_addr
                ));
                if let Some(comm) = self.communicator.as_mut() {
                    comm.shutdown();
                }
                self.communicator = None;
                self.logger.warn("Retrying to reconnect to the server ...");

                let msg_data_cloned = msg_data.clone();

                // --- KEEP ALIVE TIMER ---
                // Cancela uno anterior si existe
                if let Some(handle) = self.keep_alive_timer.take() {
                    ctx.cancel_future(handle);
                }
                // Programa un timer dummy para mantener vivo el actor
                let keep_alive_handle = ctx.run_interval(Duration::from_secs(1), |_, _| {
                    // No hace nada, solo mantiene vivo el actor
                    println!("[Delivery][KeepAlive] Manteniendo actor vivo durante reconexión");
                });
                self.keep_alive_timer = Some(keep_alive_handle);

                // --- TIMER DE RECONEXIÓN ---
                let handle = ctx.run_later(DELAY_SECONDS_TO_START_RECONNECT, move |_, ctx| {
                    println!("Attempting to reconnect after delay...");
                    ctx.address().do_send(ConnectionClosed {
                        remote_addr: msg_data_cloned.remote_addr,
                    });
                    println!("Reconnection attempt sent.");
                });
                self.waiting_reconnection_timer = Some(handle);
            }
            _ => {
                self.logger
                    .info(format!("NetworkMessage ignored: {:?}", msg));
            }
        }
    }
}

impl Drop for Delivery {
    fn drop(&mut self) {
        actix::System::current().stop();
        process::exit(0);
    }
}
