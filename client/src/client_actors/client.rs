use crate::client_actors::ui_handler::UIHandler;
use crate::messages::internal_messages::*;
use actix::fut::wrap_future;
use actix::prelude::*;
use colored::Color;
use common::constants::BASE_DELAY_MILLIS;
use common::constants::DELAY_SECONDS_TO_START_RECONNECT;
use common::logger::Logger;
use common::messages::NearbyRestaurants;
use common::messages::OrderDelivered;
use common::messages::client_messages::*;
use common::messages::shared_messages::*;
use common::network::communicator::Communicator;
use common::network::connections::connect_one;
use common::network::connections::connect_some;
use common::network::connections::reconnect;
use common::network::peer_types::PeerType;
use common::types::dtos::ClientDTO;
use common::types::dtos::OrderDTO;
use common::types::dtos::UserDTO;
use common::types::order_status::OrderStatus;
use rand::Rng;
use std::net::SocketAddr;
use std::process;
use tokio::net::TcpStream;

/// Represents a client actor in the restaurant ordering system.
///
/// The `Client` actor manages the user's session, communicates with the server cluster,
/// handles order creation and status updates, and interacts with the UI handler.
/// It is responsible for sending and receiving network messages, tracking the current order,
/// and managing delivery timers.
pub struct Client {
    /// List of server socket addresses to connect to.
    pub servers: Vec<SocketAddr>,
    /// Unique identifier for the client.
    pub client_id: String,
    /// Current position of the client in 2D coordinates.
    pub client_position: (f32, f32),
    /// Current order placed by the client, if any.
    pub client_order: Option<OrderDTO>,
    /// Address of the UI handler actor.
    pub ui_handler: Option<Addr<UIHandler>>,
    /// Communicator for network interactions with the server.
    pub communicator: Option<Communicator<Client>>,
    /// Pending TCP stream before the actor starts.
    pub pending_stream: Option<TcpStream>,
    /// Logger for client events.
    pub logger: Logger,
    /// Handle for the delivery timer, if active.
    delivery_timer: Option<actix::SpawnHandle>,
    /// Timer for waiting reconnection attempts after a connection is closed.
    waiting_reconnection_timer: Option<actix::SpawnHandle>,
    /// Flag to indicate if the client is already connected and waiting for reconnection.
    already_connected: bool,
}

impl Client {
    /// Creates a new `Client` instance, connecting to one of the available servers.
    ///
    /// ## Arguments
    ///
    /// * `servers` - A vector of server socket addresses.
    /// * `client_id` - The unique identifier for the client.
    /// * `client_position` - The initial position of the client.
    ///
    /// ## Returns
    ///
    /// Returns a new `Client` instance.
    pub async fn new(
        servers: Vec<SocketAddr>,
        client_id: String,
        client_position: (f32, f32),
    ) -> Self {
        let logger = Logger::new(format!("Client {}", &client_id), Color::Cyan);
        logger.info(format!("Hello, {}!", client_id));
        let pending_stream = connect_some(servers.clone(), PeerType::ClientType).await;

        if pending_stream.is_none() {
            logger.error("Failed to connect to any server. Exiting.");
            std::process::exit(1);
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
            delivery_timer: None, // Inicializamos el temporizador de entrega como None
            waiting_reconnection_timer: None, // Timer for reconnection attempts
            already_connected: false, // Flag to indicate if waiting for reconnection
        }
    }

    /// Sends a network message to the connected server via the communicator.
    ///
    /// ## Arguments
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
            self.logger.error("Communicator not found!");
        }
    }

    /// Starts the client logic by requesting the current leader from the server
    /// (Sends a WhoIsLeader message).
    ///
    /// ## Arguments
    ///
    /// * `_ctx` - The Actix actor context.
    pub fn start_running(&self, _ctx: &mut Context<Self>) {
        let actual_socket_addr = self
            .communicator
            .as_ref()
            .map(|c| c.local_address)
            .expect("Socket address not initialized");
        self.logger.info(format!(
            "Starting Client actor with ID: {} at position: {:?}",
            self.client_id, self.client_position
        ));

        self.send_network_message(NetworkMessage::WhoIsLeader(WhoIsLeader {
            origin_addr: actual_socket_addr,
            user_id: self.client_id.clone(),
        }));
    }

    /// Manages the delivery timer for the current order.
    ///
    /// If the order is in the `Delivering` state, a timer is started to track the expected delivery time.
    /// If the order is delivered or cancelled, the timer is cancelled.
    ///
    /// ## Arguments
    ///
    /// * `order` - The order to track.
    /// * `ctx` - The Actix actor context.
    pub fn manage_delivery_time(&mut self, order: &OrderDTO, ctx: &mut actix::Context<Self>) {
        if order.client_id != self.client_id {
            self.logger.warn(format!(
                "Received delivery expected time for a different client ({}), ignoring",
                order.client_id
            ));
            return;
        }

        if order.status == OrderStatus::Delivering {
            self.logger.info(format!(
                "Estimated delivery time for your order: {:.2} seconds.",
                order.expected_delivery_time as f64 / 1000.0
            ));

            // Cancelar timer anterior si existe
            if let Some(handle) = self.delivery_timer.take() {
                ctx.cancel_future(handle);
            }

            let delivery_time = order.expected_delivery_time / 1000 + BASE_DELAY_MILLIS;
            let handle = ctx.run_later(
                //////////////////////////////////////////////
                // TODO: IF YA LLEGÓ EL PEDIDO, NO HAY QUE ESPERAR, HACER UN SHUTDOWN ENTERO
                std::time::Duration::from_secs(delivery_time),
                move |act, _ctx| {
                    act.logger.info(format!(
                        "Delivery expected time of {:.2} seconds has elapsed!",
                        delivery_time as f64
                    ));
                    if let Some(order) = &act.client_order {
                        if order.status == OrderStatus::Delivered {
                            act.logger.info(format!(
                                "Order {} has been delivered successfully.",
                                order.order_id
                            ));
                        } else {
                            act.logger.warn(
                                "Expected delivery time elapsed, sending order has been delivered",
                            );
                            let mut order = order.clone();
                            order.status = OrderStatus::Delivered;
                            act.client_order = Some(order.clone());
                            act.send_network_message(NetworkMessage::OrderDelivered(
                                OrderDelivered {
                                    order: order.clone(),
                                },
                            ));
                        }
                    } else {
                        act.logger.warn("No active order found for delivery check.");
                    }
                    act.delivery_timer = None;
                },
            );
            self.delivery_timer = Some(handle);
        } else {
            // Cancelar timer si el estado ya no es Delivering
            if let Some(handle) = self.delivery_timer.take() {
                ctx.cancel_future(handle);
            }
        }
    }
}

/// Handles [`ConnectionClosed`] messages.
///
/// This handler is triggered when the connection to the server is lost.
/// It attempts to reconnect to one of the known servers. If reconnection is successful,
/// it reinitializes the communicator and restarts the actor. If not, the actor is stopped.
impl Handler<ConnectionClosed> for Client {
    type Result = ();

    fn handle(&mut self, _msg: ConnectionClosed, ctx: &mut Self::Context) -> Self::Result {
        // Llama a la función async y usa wrap_future para obtener el resultado
        // Antes de reconectar o crear communicator:

        if let Some(handle) = self.delivery_timer.take() {
            ctx.cancel_future(handle);
            self.delivery_timer = None;
        }

        if self.communicator.is_some() {
            self.logger
                .info("Already connected, skipping reconnection.");
            return;
        }

        let servers = self.servers.clone();
        let fut = async move { reconnect(servers, PeerType::ClientType).await };

        let fut = wrap_future::<_, Self>(fut).map(|result, actor: &mut Self, ctx| match result {
            Some(stream) => {
                let communicator = Communicator::new(stream, ctx.address(), PeerType::ClientType);
                actor.communicator = Some(communicator);

                actor
                    .logger
                    .info("Reconnected successfully. Restarting actor...");

                if let Some(handler) = actor.waiting_reconnection_timer.take() {
                    ctx.cancel_future(handler);
                    actor.waiting_reconnection_timer = None;
                }
                // Esperar 100ms antes de enviar WhoIsLeader tras reconexión
                let addr = ctx.address();
                let handler = ctx.run_later(std::time::Duration::from_millis(100), move |_, _| {
                    addr.do_send(StartRunning);
                });
                actor.waiting_reconnection_timer = Some(handler);
            }
            None => {
                actor
                    .logger
                    .error("Failed to reconnect to any server after closed connection");
                ctx.stop();
            }
        });
        ctx.spawn(fut);
    }
}

impl Actor for Client {
    type Context = Context<Self>;

    /// Called when the `Client` actor is started.
    ///
    /// Initializes the network communicator and the UI handler, then begins the client logic
    /// by requesting the current leader from the server cluster.
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
            PeerType::ClientType,
        );
        self.communicator = Some(communicator);

        let ui_handler = UIHandler::new(ctx.address(), self.logger.clone());
        self.ui_handler = Some(ui_handler.start());

        // Esperar 100ms antes de enviar WhoIsLeader
        let addr = ctx.address();
        let handler = ctx.run_later(std::time::Duration::from_millis(100), move |_, _| {
            addr.do_send(StartRunning);
        });
        self.waiting_reconnection_timer = Some(handler);
    }
}

impl Handler<StartRunning> for Client {
    type Result = ();
    fn handle(&mut self, _msg: StartRunning, ctx: &mut Self::Context) -> Self::Result {
        self.start_running(ctx);
    }
}

/// Handler for the `LeaderIs` message.
///
/// Handles notification of the current leader's address. If not already connected to the leader,
/// attempts to establish a new connection and updates the communicator.
impl Handler<LeaderIs> for Client {
    type Result = ();

    fn handle(&mut self, msg: LeaderIs, ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Received new leader address: {}", msg.coord_addr));
        let leader_addr = msg.coord_addr;
        let self_addr = ctx.address();
        let logger = self.logger.clone();

        if let Some(handler) = self.waiting_reconnection_timer.take() {
            ctx.cancel_future(handler);
            self.waiting_reconnection_timer = None;
        }

        let communicator_opt = self.communicator.as_ref().map(|c| c.peer_address);

        // Si ya estamos conectados al líder, no hacemos nada
        if Some(leader_addr) == communicator_opt {
            self.logger.info(format!(
                "Already connected to the leader at address: {}",
                leader_addr.clone()
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

        // Si no estoy conectado al líder, cierro el communicator anterior y conecto al nuevo líder
        if let Some(comm) = self.communicator.as_mut() {
            comm.shutdown();
        }
        self.communicator = None;

        ctx.spawn(
            wrap_future(async move {
                logger.info(format!(
                    "Attempting to connect to the new leader at address: {}",
                    leader_addr
                ));
                if let Some(new_stream) = connect_one(leader_addr, PeerType::ClientType).await {
                    let new_communicator =
                        Communicator::new(new_stream, self_addr.clone(), PeerType::ClientType);
                    Some(new_communicator)
                } else {
                    logger.error(format!(
                        "Failed to connect to the new leader at {}",
                        leader_addr
                    ));
                    None
                }
            })
            .map(move |maybe_communicator, actor: &mut Self, ctx| {
                if let Some(new_communicator) = maybe_communicator {
                    actor.logger.info(format!(
                        "Communicator updated with new peer address: {}",
                        new_communicator.peer_address
                    ));
                    actor.communicator = Some(new_communicator);

                    // Usar ctx.address() directamente
                    let handler =
                        ctx.run_later(std::time::Duration::from_millis(100), move |_, ctx| {
                            ctx.address().do_send(StartRunning);
                        });
                    actor.waiting_reconnection_timer = Some(handler);
                }
            }),
        );
    }
}

/// Handler for the `RecoverProcedure` message.
///
/// Restores the client's state from a recovery message, including position and current order.
/// If there is no active order, requests nearby restaurants.
impl Handler<RecoverProcedure> for Client {
    type Result = ();

    fn handle(&mut self, msg: RecoverProcedure, ctx: &mut Self::Context) -> Self::Result {
        if self.already_connected {
            self.logger
                .info("Already connected, skipping recovery procedure.");
            return;
        }
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
                                self.logger.warn(format!(
                                    "Your order has been cancelled: {}. Try again later.",
                                    client_dto.dish_name
                                ));
                                ctx.stop();
                            }
                            OrderStatus::Delivered => {
                                self.logger.info(format!(
                                    "Your order has already been delivered: {}. Enjoy your meal!",
                                    client_dto.dish_name
                                ));
                                ctx.stop();
                            }
                            _ => {
                                self.logger
                                    .info(format!("State after recovering: {}", client_dto.status));
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
                    self.already_connected = true;
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

/// Handler for the `SendThisOrder` message.
///
/// Creates a new order with the selected restaurant and dish, and sends it to the server.
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
            expected_delivery_time: 0,    // Tiempo de entrega inicial
        };

        // Enviar el pedido al servidor
        let network_message = NetworkMessage::RequestThisOrder(RequestThisOrder { order });
        self.send_network_message(network_message);
    }
}

/// Handler for the `NearbyRestaurants` message.
///
/// Forwards the list of nearby restaurants to the UI handler for user selection.
impl Handler<NearbyRestaurants> for Client {
    type Result = ();

    fn handle(&mut self, msg: NearbyRestaurants, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(ui_handler) = &self.ui_handler {
            ui_handler.do_send(SelectNearbyRestaurants {
                nearby_restaurants: msg.restaurants,
            });
        } else {
            self.logger.error("UIHandler not initialized");
        }
    }
}

/// Handles [`NetworkMessage`] messages.
///
/// This is the main entry point for all network messages received by the client actor.
/// It matches on the message variant and dispatches logic accordingly, such as handling recovered state,
/// new orders, delivery updates, and order finalization.
impl Handler<NetworkMessage> for Client {
    type Result = ();
    fn handle(&mut self, msg: NetworkMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NetworkMessage::RetryLater(_msg_data) => {
                self.logger.info("Retrying to connect in some seconds");
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
                self.already_connected = true;
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
            NetworkMessage::CancelOrder(msg_data) => {
                // Chequeo si el pedido es el mio
                if let Some(order) = &self.client_order {
                    if order.order_id == msg_data.order.order_id {
                        self.logger
                            .info("Your order has been cancelled. Try again later.");
                        self.client_order = None; // Limpiamos el pedido actual
                        ctx.stop();
                    } else {
                        self.logger.error(format!(
                            "Received cancel request for order {}, but I have order {}",
                            msg_data.order.order_id, order.order_id
                        ));
                    }
                } else {
                    self.logger
                        .warn("No restaurants found for the order. Try again later.");
                        ctx.stop();
                    }
            }

            NetworkMessage::NotifyOrderUpdated(msg_data) => {
                self.logger.info(format!(
                    "Your order is now: {:?}",
                    msg_data.order.status.to_string().to_uppercase()
                ));
                self.client_order = Some(msg_data.order.clone());
                match msg_data.order.status {
                    OrderStatus::Delivered => {
                        self.logger
                            .info("Your order has been delivered. Thanks for using our service!");
                        ctx.stop();
                    }
                    OrderStatus::Unauthorized => {
                        self.logger
                            .info("Your order has been unauthorized. Please try again later.");
                        ctx.stop();
                    }
                    OrderStatus::Cancelled => {
                        self.logger.info(format!(
                            "Order {} has been cancelled. Please try again later.",
                            msg_data.order.order_id
                        ));
                        ctx.stop();
                    }

                    _ => {}
                }
                self.manage_delivery_time(&msg_data.order, ctx);
            }

            NetworkMessage::ConnectionClosed(msg_data) => {
                self.logger.info(format!(
                    "Connection closed with address: {}",
                    msg_data.remote_addr
                ));

                // CANCELA EL TIMER DE DELIVERY SI EXISTE
                if let Some(handle) = self.delivery_timer.take() {
                    ctx.cancel_future(handle);
                    self.delivery_timer = None;
                }

                // Si el comunicador actual posee una peer_address que coincide con la dirección cerrada,
                // se elimina el comunicador actual. Si no, se ignora.

                self.logger.info(format!(
                    "Removing communicator for address: {}",
                    msg_data.remote_addr
                ));
                self.communicator = None;
                self.logger.warn("Retrying to reconnect to the server ...");

                let msg_data_cloned = msg_data.clone();

                // Inicia un temporizador para reconectar después de un tiempo
                let handle = ctx.run_later(DELAY_SECONDS_TO_START_RECONNECT, move |_, ctx| {
                    ctx.address().do_send(ConnectionClosed {
                        remote_addr: msg_data_cloned.remote_addr,
                    });
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

impl Drop for Client {
    fn drop(&mut self) {
        actix::System::current().stop();
        process::exit(0);
    }
}
