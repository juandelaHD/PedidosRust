use crate::internal_messages::messages::SendToKitchen;
use crate::restaurant_actors::delivery_assigner::DeliveryAssigner;
use crate::restaurant_actors::kitchen::Kitchen;
use actix::fut::wrap_future;
use actix::prelude::*;
use colored::Color;
use common::constants::DELAY_SECONDS_TO_START_RECONNECT;
use common::logger::Logger;
use common::messages::{
    ConnectionClosed, DeliverThisOrder, DeliveryAccepted, LeaderIs, NetworkMessage, NewOrder,
    RecoverProcedure, RegisterUser, RequestNearbyDelivery, StartRunning, UpdateOrderStatus,
    WhoIsLeader,
};
use common::network::communicator::Communicator;
use common::network::connections::{connect_one, connect_some, reconnect};
use common::network::peer_types::PeerType;
use common::types::dtos::{OrderDTO, UserDTO};
use common::types::order_status::OrderStatus;
use common::types::restaurant_info::RestaurantInfo;
use common::utils::random_bool_by_given_probability;
use std::collections::HashSet;
use std::net::SocketAddr;
use tokio::net::TcpStream;

/// The `Restaurant` actor represents a restaurant in the distributed food ordering system.
///
/// ## Responsibilities
/// - Registers itself with the server cluster.
/// - Receives and processes new orders from clients.
/// - Forwards orders to the kitchen for preparation.
/// - Handles delivery assignment and order status updates.
/// - Manages reconnection and recovery scenarios.
pub struct Restaurant {
    /// Basic information about the restaurant.
    pub info: RestaurantInfo,
    /// Probability for accepting or rejecting an order.
    pub probability: f32,
    /// Address of the kitchen actor.
    pub kitchen_address: Option<Addr<Kitchen>>,
    /// Address of the delivery assigner actor.
    pub delivery_assigner_address: Option<Addr<DeliveryAssigner>>,
    /// Network communicator for server interaction.
    pub communicator: Option<Communicator<Restaurant>>,
    /// Pending TCP stream before the actor starts.
    pub pending_stream: Option<TcpStream>,
    /// Logger for restaurant events.
    pub logger: Logger,
    /// List of server socket addresses.
    pub servers: Vec<SocketAddr>,
    waiting_reconnection_timer: Option<actix::SpawnHandle>,
}

impl Restaurant {
    /// Creates a new `Restaurant` actor instance.
    ///
    /// # Arguments
    /// * `info` - Basic information about the restaurant.
    /// * `probability` - Probability fo accepting or rejecting an order.
    /// * `servers` - List of server socket addresses.
    /// * `logger` - Logger for restaurant events.
    pub async fn new(info: RestaurantInfo, probability: f32, servers: Vec<SocketAddr>) -> Self {
        let logger = Logger::new("Restaurant", Color::BrightGreen);
        logger.info(format!("Hello: {}'s restaurant!", info.id));
        // Intentamos conectarnos a los servidores
        let pending_stream = connect_some(servers.clone(), PeerType::RestaurantType).await;

        if pending_stream.is_none() {
            logger.error(format!(
                "Failed to connect to any server from the list: {:?}",
                servers
            ));
            std::process::exit(1);
        }

        Self {
            info,
            probability,
            kitchen_address: None,
            delivery_assigner_address: None,
            communicator: None,
            pending_stream,
            logger,
            servers,
            waiting_reconnection_timer: None,
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

/// Handles [`ConnectionClosed`] messages.
///
/// This handler is triggered when the connection to the server is lost.
/// It attempts to reconnect to one of the known servers. If reconnection is successful,
/// it reinitializes the communicator and restarts the actor. If not, the actor is stopped.
impl Handler<ConnectionClosed> for Restaurant {
    type Result = ();

    fn handle(&mut self, _msg: ConnectionClosed, ctx: &mut Self::Context) -> Self::Result {
        // Llama a la función async y usa wrap_future para obtener el resultado
        // Antes de reconectar o crear communicator:
        if self.communicator.is_some() {
            self.logger
                .info("Already connected, skipping reconnection.");
            return;
        }

        let servers = self.servers.clone();
        let fut = async move { reconnect(servers, PeerType::RestaurantType).await };

        let fut = wrap_future::<_, Self>(fut).map(|result, actor: &mut Self, ctx| match result {
            Some(stream) => {
                let communicator =
                    Communicator::new(stream, ctx.address(), PeerType::RestaurantType);
                actor.communicator = Some(communicator);

                actor.delivery_assigner_address =
                    Some(DeliveryAssigner::new(actor.info.clone(), ctx.address()).start());

                actor.kitchen_address = Some(
                    Kitchen::new(
                        ctx.address(),
                        actor.delivery_assigner_address.clone().unwrap(),
                    )
                    .start(),
                );

                actor
                    .logger
                    .info("Reconnected successfully. Restarting actor...");

                if let Some(handler) = actor.waiting_reconnection_timer.take() {
                    ctx.cancel_future(handler);
                    actor.waiting_reconnection_timer = None;
                }

                // Esperar 100ms antes de enviar WhoIsLeader tras reconexión
                let addr = ctx.address();
                ctx.run_later(std::time::Duration::from_millis(100), move |_, _| {
                    addr.do_send(StartRunning);
                });
            }
            None => {
                actor.logger.error(format!(
                    "Failed to reconnect to any server after closed connection"
                ));
                ctx.stop();
            }
        });
        ctx.spawn(fut);
    }
}

impl Handler<StartRunning> for Restaurant {
    type Result = ();
    fn handle(&mut self, _msg: StartRunning, ctx: &mut Self::Context) -> Self::Result {
        self.start_running(ctx);
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
            PeerType::RestaurantType,
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

/// Handles [`LeaderIs`] messages.
///
/// This handler is called when the server notifies the restaurant of the current leader's address.
/// If already connected to the leader, it registers itself. Otherwise, it attempts to connect to the new leader
/// and updates its communicator accordingly.
impl Handler<LeaderIs> for Restaurant {
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
                origin_addr: local_address.clone(),
                user_id: self.info.id.clone(),
                position: self.info.position,
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
                if let Some(new_stream) = connect_one(leader_addr, PeerType::RestaurantType).await {
                    let new_communicator =
                        Communicator::new(new_stream, self_addr.clone(), PeerType::RestaurantType);
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
                    ctx.run_later(std::time::Duration::from_millis(100), move |_, ctx| {
                        ctx.address().do_send(StartRunning);
                    });
                }
            }),
        );
    }
}

/// Handles [`RecoverProcedure`] messages.
///
/// This handler is used to recover the restaurant's state after a reconnection or failure.
/// It updates the restaurant's position and replays all pending and authorized orders to the kitchen.
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

/// Handles [`NewOrder`] messages.
///
/// Processes a new order received from the server. If the order is pending, it forwards it to the kitchen.
/// If the order is authorized, it randomly accepts or rejects it based on the restaurant's probability,
/// then updates the order status accordingly.
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

/// Handles [`UpdateOrderStatus`] messages.
///
/// Forwards an order status update to the server cluster via the network communicator.
impl Handler<UpdateOrderStatus> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: UpdateOrderStatus, _ctx: &mut Self::Context) -> Self::Result {
        self.send_network_message(NetworkMessage::UpdateOrderStatus(msg));
    }
}

/// Handles [`RequestNearbyDelivery`] messages.
///
/// Sends a request to the server to find available delivery personnel for a given order.
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

/// Handles [`DeliveryAccepted`] messages.
///
/// Notifies the server that a delivery person has accepted the delivery for a specific order.
impl Handler<DeliveryAccepted> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: DeliveryAccepted, _ctx: &mut Self::Context) -> Self::Result {
        self.send_network_message(NetworkMessage::DeliveryAccepted(msg));
    }
}

/// Handles [`DeliverThisOrder`] messages.
///
/// Forwards the delivery assignment to the server, indicating that the delivery
/// has to proceed.
impl Handler<DeliverThisOrder> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: DeliverThisOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.send_network_message(NetworkMessage::DeliverThisOrder(msg));
    }
}

/// Handles [`NetworkMessage`] messages.
///
/// This is the main entry point for all network messages received by the restaurant actor.
/// It matches on the message variant and dispatches logic accordingly, such as handling recovered state,
/// new orders, delivery updates, and order finalization.
impl Handler<NetworkMessage> for Restaurant {
    type Result = ();
    fn handle(&mut self, msg: NetworkMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            // All Users messages
            NetworkMessage::RetryLater(_msg_data) => {
                self.logger.info("Retrying to connect in some seconds");

                // let ctx_addr = ctx.address();
                // let handle = ctx.spawn(
                //     actix::clock::sleep(std::time::Duration::from_secs(3)).into_actor(self).map(move |_, _, ctx| {
                //         // aca hay que llamar a start_running
                //     })
                // );
                // self.waiting_reconnection_timer= Some(handle);
            }
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
            NetworkMessage::DeliveryAvailable(msg_data) => {
                if let Some(addr) = self.delivery_assigner_address.as_ref() {
                    addr.do_send(msg_data);
                }
            }
            NetworkMessage::CancelOrder(msg_data) => {
                self.logger.info(format!(
                    "Order with ID: {} has been cancelled.",
                    msg_data.order.order_id
                ));
                if let Some(addr) = self.delivery_assigner_address.as_ref() {
                    addr.do_send(msg_data);
                }
            }

            NetworkMessage::ConnectionClosed(msg_data) => {
                self.logger.info(format!(
                    "Connection closed with address: {}",
                    msg_data.remote_addr
                ));
                // Si el comunicador actual posee una peer_address que coincide con la dirección cerrada,
                // se elimina el comunicador actual. Si no, se ignora.
                if let Some(communicator) = &self.communicator {
                    if communicator.peer_address == msg_data.remote_addr {
                        self.logger.info(format!(
                            "Removing communicator for address: {}",
                            msg_data.remote_addr
                        ));
                        self.communicator = None;
                        self.logger.warn("Retrying to reconnect to the server ...");

                        let msg_data_cloned = msg_data.clone();

                        // Inicia un temporizador para reconectar después de un tiempo
                        let handle =
                            ctx.run_later(DELAY_SECONDS_TO_START_RECONNECT, move |_, ctx| {
                                ctx.address().do_send(msg_data_cloned.clone());
                            });

                        self.waiting_reconnection_timer = Some(handle);
                        // let msg_clone = msg_data.clone();
                        // let handle = ctx.spawn(
                        //     actix::clock::sleep(std::time::Duration::from_secs(3))
                        //         .into_actor(self)
                        //         .map(move |_, _, ctx| {
                        //             ctx.address().do_send(msg_clone.clone());
                        //         }),
                        // );
                    }
                } else {
                    self.logger
                        .error("Communicator not found, cannot handle closed connection.");
                }
            }

            _ => {
                self.logger
                    .info(format!("NetworkMessage ignored: {:?}", msg));
            }
        }
    }
}

impl Drop for Restaurant {
    fn drop(&mut self) {
        println!("[Restaurant] ACTOR DROPPED");
    }
}
