use crate::messages::internal_messages::RegisterConnection;
use crate::messages::internal_messages::RegisterConnectionWithCoordinator;
use crate::messages::internal_messages::SetActorsAddresses;
use crate::server_actors::coordinator_manager::CoordinatorManager;
use crate::server_actors::services::nearby_delivery::NearbyDeliveryService;
use crate::server_actors::services::nearby_restaurants::NearbyRestaurantsService;
use crate::server_actors::services::orders_services::OrderService;
use crate::server_actors::storage::Storage;
use actix::prelude::*;
use colored::Color;
use common::bimap::BiMap;
use common::constants::BASE_PORT;
use common::logger::Logger;
use common::messages::CancelOrder;
use common::messages::DeliverThisOrder;
use common::messages::OrderFinalized;
use common::messages::UpdateOrderStatus;
use common::messages::coordinator_messages::*;
use common::messages::internal_messages::*;
use common::messages::shared_messages::*;
use common::network::communicator::Communicator;
use common::network::connections::connect_to_all;
use common::network::peer_types::PeerType;
use common::types::delivery_status::DeliveryStatus;
use common::types::dtos::ClientDTO;
use common::types::dtos::DeliveryDTO;
use common::types::dtos::OrderDTO;
use common::types::dtos::RestaurantDTO;
use common::types::dtos::UserDTO;
use common::types::order_status::OrderStatus;
use std::collections::HashSet;
use std::time::Duration;
use std::{collections::HashMap, net::SocketAddr};
use tokio::net::TcpStream;

/// The `Coordinator` actor orchestrates the main logic of the distributed system,
/// managing user connections, order processing, communication with other coordinators,
/// and coordination with services such as storage, order management, and nearby services.
///
/// ## Responsibilities
/// - Registers and manages user and peer connections.
/// - Handles incoming and outgoing network messages.
/// - Coordinates order assignment, delivery, and cancellation.
/// - Manages communication with the `CoordinatorManager` for leader election and storage updates.
/// - Interfaces with the `OrderService`, `NearbyRestaurantsService`, and `NearbyDeliveryService`.
#[derive(Debug)]
pub struct Coordinator {
    /// Unique identifier for this coordinator.
    pub id: String,
    /// Addresses of all nodes in the ring.
    pub ring_nodes: HashMap<String, SocketAddr>,
    /// Socket address of this coordinator.
    pub my_addr: SocketAddr,
    /// Current coordinator's address (leader).
    pub current_coordinator: Option<SocketAddr>,
    /// Bi-directional map of user addresses and user IDs.
    pub user_addresses: BiMap<SocketAddr, String>,
    /// Map of remote addresses to their communicators.
    pub communicators: HashMap<SocketAddr, Communicator<Coordinator>>,
    /// Address of the storage actor.
    pub storage: Option<Addr<Storage>>,
    /// Address of the order service actor.
    pub order_service: Option<Addr<OrderService>>,
    /// Address of the nearby restaurants service actor.
    pub nearby_restaurant_service: Option<Addr<NearbyRestaurantsService>>,
    /// Address of the nearby delivery service actor.
    pub nearby_delivery_service: Option<Addr<NearbyDeliveryService>>,
    /// Logger for coordinator events.
    pub logger: Logger,
    /// Address of the coordinator manager actor.
    pub coordinator_manager: Option<Addr<CoordinatorManager>>,
    /// Pending TCP streams for ring connections.
    pub pending_streams: HashMap<SocketAddr, TcpStream>,
    /// Timers for order assignment timeouts.
    pub order_timers: HashMap<u64, SpawnHandle>,
}

impl Coordinator {
    /// Asynchronously creates a new `Coordinator` instance, connects to ring nodes,
    /// and initializes the logger and services.
    ///
    /// ## Arguments
    /// * `srv_addr` - The socket address of this coordinator.
    /// * `ring_nodes` - Map of all ring node IDs to their addresses.
    pub async fn new(srv_addr: SocketAddr, ring_nodes: HashMap<String, SocketAddr>) -> Self {
        // Inicializar el coordinador con la dirección del servidor y los nodos del anillo
        // y un logger compartido.
        let ring_nodes_vec: Vec<SocketAddr> = ring_nodes.values().cloned().collect();

        let pending_streams: HashMap<SocketAddr, TcpStream> =
            connect_to_all(ring_nodes_vec, PeerType::CoordinatorType).await;

        if pending_streams.is_empty() {
            println!("No connections established.");
        }
        Self {
            id: format!("server_{}", srv_addr.port() - BASE_PORT),
            ring_nodes,
            my_addr: srv_addr,
            current_coordinator: None,
            user_addresses: BiMap::new(),
            logger: Logger::new("COORDINATOR", Color::Black),
            coordinator_manager: None,
            communicators: HashMap::new(),
            pending_streams,
            order_service: Some(OrderService::new().await.start()),
            nearby_restaurant_service: None,
            nearby_delivery_service: None,
            storage: None,
            order_timers: HashMap::new(),
        }
    }

    /// Sends a [`NetworkMessage`] to a user by their user ID.
    ///
    /// ## Arguments
    /// * `user_id` - The user ID to send the message to.
    /// * `message` - The [`NetworkMessage`] to send.
    pub fn send_network_message(&self, user_id: String, message: NetworkMessage) {
        if let Some(user_addr) = self.user_addresses.get_by_value(&user_id).cloned() {
            if let Some(communicator) = self.communicators.get(&user_addr) {
                if let Some(sender) = &communicator.sender {
                    sender.do_send(message);
                } else {
                    self.logger.info(format!("No sender found for {}", user_id));
                }
            } else {
                self.logger
                    .info(format!("Communicator not found for {}", user_id));
            }
        } else {
            self.logger.info(format!("User ID {} not found", user_id));
        }
    }

    /// Broadcasts delivery offers to all available delivery agents for a given order,
    /// and starts a timer to cancel the order if not accepted in time.
    ///
    /// ## Arguments
    /// * `order` - The [`OrderDTO`] to be delivered.
    /// * `deliveries` - List of [`DeliveryDTO`]s representing available delivery agents.
    /// * `ctx` - The actor context.
    pub fn broadcast_deliveries(
        &mut self,
        order: OrderDTO,
        deliveries: Vec<DeliveryDTO>,
        ctx: &mut Context<Self>,
    ) {
        for delivery in deliveries {
            if let Some(delivery_addr) = self
                .user_addresses
                .get_by_value(&delivery.delivery_id)
                .cloned()
            {
                if let Some(communicator) = self.communicators.get(&delivery_addr) {
                    if let Some(sender) = &communicator.sender {
                        sender.do_send(NetworkMessage::NewOfferToDeliver(NewOfferToDeliver {
                            order: order.clone(),
                        }));
                    } else {
                        self.logger
                            .info(format!("No sender found for {}", delivery.delivery_id));
                    }
                } else {
                    self.logger.info(format!(
                        "Communicator not found for {}",
                        delivery.delivery_id
                    ));
                }
            } else {
                self.logger
                    .info(format!("User ID {} not found", delivery.delivery_id));
            }
        }

        // Iniciar timer para la orden
        let order_id = order.order_id;
        let timer_duration = Duration::from_secs(30); // Por ejemplo, 30 segundos

        let handle = ctx.run_later(timer_duration, move |actor, _ctx| {
            actor.logger.warn(format!(
                "Order {} timed out, no delivery accepted.",
                order_id
            ));
            _ctx.address().do_send(CancelOrder {
                order: OrderDTO {
                    order_id,
                    client_id: order.client_id.clone(),
                    dish_name: order.dish_name.clone(),
                    restaurant_id: order.restaurant_id.clone(),
                    status: OrderStatus::Cancelled,
                    delivery_id: None,
                    client_position: order.client_position,
                    expected_delivery_time: 0,
                    time_stamp: std::time::SystemTime::now(),
                },
            });
            actor.order_timers.remove(&order_id);
        });

        self.order_timers.insert(order_id, handle);
    }

    /// Handles the acceptance of an order by a delivery agent, cancelling the assignment timer.
    ///
    /// ## Arguments
    /// * `order_id` - The ID of the accepted order.
    /// * `ctx` - The actor context.
    fn handle_order_accepted(&mut self, order_id: u64, ctx: &mut Context<Self>) {
        if let Some(handle) = self.order_timers.remove(&order_id) {
            ctx.cancel_future(handle);
            self.logger
                .info(format!("Order {} accepted, timer cancelled.", order_id));
        }
    }
}

impl Actor for Coordinator {
    type Context = Context<Self>;

    /// Initializes storage, services, and coordinator manager when the actor starts.
    fn started(&mut self, ctx: &mut Self::Context) {
        // Inicializar el servicio de almacenamiento
        let storage = Storage::new(ctx.address());
        let storage_address = storage.start();
        self.storage = Some(storage_address.clone());

        let coordinator_manager = CoordinatorManager::new(
            self.id.clone(),
            self.my_addr,
            self.ring_nodes.clone(),
            ctx.address(),
            storage_address.clone(),
        );

        self.coordinator_manager = Some(coordinator_manager.start());
        self.logger.info("Coordinator started.");

        for (addr, stream) in self.pending_streams.drain() {
            let communicator = Communicator::new(stream, ctx.address(), PeerType::CoordinatorType);
            // le paso los coordinadores que hay al CoordinatorManager
            if let Some(coordinator_manager) = &self.coordinator_manager {
                coordinator_manager.do_send(RegisterConnectionWithCoordinator {
                    remote_addr: addr,
                    communicator,
                });
            } else {
                self.logger
                    .info("CoordinatorManager not initialized yet, cannot register connection.");
            }
        }

        // Enviar un startRunning al CoordinatorManager
        if let Some(coordinator_manager) = &self.coordinator_manager {
            coordinator_manager.do_send(StartRunning);
        } else {
            self.logger
                .info("CoordinatorManager not initialized yet, cannot start running.");
        }

        // Inicializar el servicio de restaurantes cercanos
        let nearby_restaurant_service =
            NearbyRestaurantsService::new(storage_address.clone(), ctx.address());
        self.nearby_restaurant_service = Some(nearby_restaurant_service.start());
        // Inicializar el servicio de delivery cercanos
        let nearby_delivery_service =
            NearbyDeliveryService::new(storage_address.clone(), ctx.address());
        self.nearby_delivery_service = Some(nearby_delivery_service.start());

        if let Some(order_service) = &self.order_service {
            order_service.do_send(SetActorsAddresses {
                coordinator_addr: ctx.address(),
                storage_addr: storage_address,
            });
        }

        self.logger.info("Services initialized.");
    }
}

/// Handles registration of a new coordinator connection.
impl Handler<RegisterConnectionWithCoordinator> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: RegisterConnectionWithCoordinator, _ctx: &mut Context<Self>) {
        // si recibi un nuevo coordinador se lo pasamos al CoordinatorManager
        if let Some(coordinator_manager) = &self.coordinator_manager {
            coordinator_manager.do_send(msg);
        } else {
            self.logger
                .info("CoordinatorManager not initialized yet, cannot register connection.");
        }
        // Aquí podrías enviar un mensaje de bienvenida o iniciar alguna lógica adicional
    }
}

/// Handles registration of a new client/peer connection.
impl Handler<RegisterConnection> for Coordinator {
    type Result = ();
    fn handle(&mut self, msg: RegisterConnection, _ctx: &mut Self::Context) -> Self::Result {
        // Registrar la conexión del cliente
        self.communicators.insert(msg.client_addr, msg.communicator);

        // TODO: El valor debe ser el ID del cliente (el nombre)
        self.user_addresses
            .insert(msg.client_addr, "UNKNOWN_USER".to_string());
        self.logger
            .info(format!("Registered connection from {} ", msg.client_addr));
    }
}

/// Handles sending a list of nearby restaurants to a client.
impl Handler<NearbyRestaurants> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: NearbyRestaurants, _ctx: &mut Self::Context) -> Self::Result {
        // Buscar el comunicador del cliente
        let client_id = msg.client.client_id.clone();
        self.send_network_message(client_id, NetworkMessage::NearbyRestaurants(msg));
    }
}

/// Handles broadcasting available deliveries to delivery agents.
impl Handler<NearbyDeliveries> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: NearbyDeliveries, ctx: &mut Self::Context) -> Self::Result {
        // Buscar el comunicador del cliente
        self.broadcast_deliveries(msg.order, msg.deliveries, ctx);
    }
}

/// Handles notifications of order updates to peers.
impl Handler<NotifyOrderUpdated> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: NotifyOrderUpdated, _ctx: &mut Self::Context) -> Self::Result {
        let peer_id = msg.peer_id.clone();
        self.send_network_message(peer_id, NetworkMessage::NotifyOrderUpdated(msg));
    }
}

/// Handles notification that a delivery agent is available for an order.
impl Handler<DeliveryAvailable> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: DeliveryAvailable, _ctx: &mut Self::Context) -> Self::Result {
        // Buscar el comunicador del delivery
        let restaurant_id = msg.order.restaurant_id.clone();
        self.send_network_message(restaurant_id, NetworkMessage::DeliveryAvailable(msg));
    }
}

/// Handles notification that a delivery agent is no longer needed for an order.
impl Handler<DeliveryNoNeeded> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: DeliveryNoNeeded, _ctx: &mut Self::Context) -> Self::Result {
        let delivery_id = msg.delivery_info.delivery_id.clone();
        self.send_network_message(delivery_id, NetworkMessage::DeliveryNoNeeded(msg));
    }
}

/// Handles instructions to deliver a specific order.
impl Handler<DeliverThisOrder> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: DeliverThisOrder, _ctx: &mut Self::Context) -> Self::Result {
        // Buscar el comunicador del delivery
        if let Some(delivery_id) = msg.order.delivery_id.clone() {
            self.logger.info(format!(
                "DeliverThisOrder received for delivery ID: {}",
                delivery_id
            ));
            self.send_network_message(delivery_id, NetworkMessage::DeliverThisOrder(msg));
        } else {
            self.logger
                .info("DeliverThisOrder received without delivery ID.");
        }
    }
}

/// Handles queries about the current leader in the system.
impl Handler<WhoIsLeader> for Coordinator {
    type Result = ();
    fn handle(&mut self, msg: WhoIsLeader, _ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Received WhoIsLeader from {}", msg.origin_addr));
        self.logger.info(format!(
            "Current coordinator: {:?}",
            self.current_coordinator
        ));
        let user_address = msg.origin_addr;

        // Si el origen está en user_addresses, actualizar el ID del usuario
        if let Some(_user_id) = self.user_addresses.get_by_key(&user_address) {
            println!("EL ORIGEN ESTA EN USER_ADDRESSES");
            self.user_addresses
                .insert(user_address, msg.user_id.clone());
            self.logger.info(format!(
                "User ID for {} is {}",
                user_address,
                msg.user_id.clone()
            ));
        } else {
            self.logger.info(format!(
                "No user ID found for {}. Adding as UNKNOWN_USER.",
                user_address
            ));
            println!("EL ORIGEN NOOO ESTA EN USER_ADDRESSES");
            // Si no está, lo actualizamos o lo agregamos
            self.user_addresses
                .insert(user_address, msg.user_id.clone());
        }

        //  Si hay un coordinador actual, se lo notificamos al cliente
        if let Some(addr) = self.current_coordinator {
            if let Some(sender) = &self.communicators[&msg.origin_addr].sender {
                println!("ENVIANDO LEADER IS A {}", msg.origin_addr);
                sender.do_send(NetworkMessage::LeaderIs(LeaderIs { coord_addr: (addr) }));
            } else {
                self.logger
                    .info(format!("No sender found for {}", msg.origin_addr));
            }
        } else {
            // Si no hay coordinador actual, le preguntamos al CoordinatorManager
            self.logger
                .info("No current coordinator available. Check Server Election implementation.");

            // coordintor no tiene lider asociado ->  enviar WhoIsLeader al CoordinatorManager
            if let Some(coordinator_manager) = &self.coordinator_manager {
                coordinator_manager.do_send(msg);
            } else {
                self.logger
                    .info("CoordinatorManager not initialized yet, cannot handle WhoIsLeader.");
            }
        }
    }
}

/// Handles updates about the current leader's ID.
impl Handler<LeaderIdIs> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: LeaderIdIs, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Received LeaderIdIs with leader ID {}",
            msg.leader_id
        ));
        if let Some(leader_addr) = self.ring_nodes.get(&msg.leader_id) {
            self.current_coordinator = Some(*leader_addr);
        } else {
            self.logger.info(format!(
                "Leader ID {} not found in ring nodes.",
                msg.leader_id
            ));
        }
    }
}

/// Handles requests to retry an operation later.
impl Handler<RetryLater> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: RetryLater, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(sender) = &self.communicators[&msg.origin_addr].sender {
            sender.do_send(NetworkMessage::RetryLater(RetryLater {
                origin_addr: self.my_addr,
            }));
        } else {
            self.logger
                .info(format!("No sender found for {}", msg.origin_addr));
        }
    }
}

/// Handles new order notifications for restaurants.
impl Handler<NewOrder> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: NewOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Received new order: {:?} for restaurant: {}",
            msg.order,
            msg.order.restaurant_id.clone()
        ));
        let restaurant_id = msg.order.restaurant_id.clone();
        self.send_network_message(restaurant_id, NetworkMessage::NewOrder(msg));
    }
}

/// Handles notifications that an order has been finalized.
impl Handler<OrderFinalized> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: OrderFinalized, _ctx: &mut Self::Context) -> Self::Result {
        let restaurant_id = msg.order.restaurant_id.clone();
        self.send_network_message(restaurant_id, NetworkMessage::OrderFinalized(msg.clone()));
    }
}

/// Handles order cancellation requests.
impl Handler<CancelOrder> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: CancelOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Received cancel order request for order ID: {}, status was: {:?}",
            msg.order.order_id, msg.order.status
        ));
        // Si el pedido esta en estado "ReadyForDelivery", le aviso al restaurante que no hay delivery (cancelando el pedido)
        if msg.order.status == OrderStatus::ReadyForDelivery {
            let restaurant_id = msg.order.restaurant_id.clone();
            self.send_network_message(restaurant_id, NetworkMessage::CancelOrder(msg.clone()));
        }
        // En cualquier otro estado, tambien le aviso al cliente
        if msg.order.status == OrderStatus::Cancelled {
            self.logger.info(format!(
                "Cancelling order {} for client {}",
                msg.order.order_id, msg.order.client_id
            ));
            self.storage.as_ref().unwrap().do_send(RemoveOrder {
                order: msg.order.clone(),
            });
            self.send_network_message(
                msg.order.restaurant_id.clone(),
                NetworkMessage::CancelOrder(msg.clone()),
            );
        }
        let client_id = msg.order.client_id.clone();
        self.send_network_message(client_id, NetworkMessage::CancelOrder(msg));
    }
}

/// Handles all incoming [`NetworkMessage`]s, dispatching them to the appropriate service or handler.
impl Handler<NetworkMessage> for Coordinator {
    type Result = ();
    fn handle(&mut self, msg: NetworkMessage, ctx: &mut Self::Context) -> Self::Result {
        println!("Received NetworkMessage: {:?}", msg);
        match msg {
            // All Users messages
            NetworkMessage::WhoIsLeader(msg_data) => {
                if self.current_coordinator.is_none() {
                    ctx.address().do_send(RetryLater {
                        origin_addr: msg_data.origin_addr,
                    });
                    return;
                }
                // Si el origen es un servidor conocido (8080, 8081, 8082, 8083), se lo paso al CoordinatorManager
                if msg_data.user_id.starts_with("server_") {
                    if let Some(coordinator_manager) = &self.coordinator_manager {
                        coordinator_manager.do_send(msg_data);
                    } else {
                        self.logger.info("CoordinatorManager not initialized yet.");
                    }
                } else {
                    // Si no es un servidor, podés manejarlo como usuario
                    self.logger.info(format!(
                        "WhoIsLeader recibido de un user: {}",
                        msg_data.origin_addr
                    ));
                    ctx.address().do_send(msg_data)
                }
            }
            NetworkMessage::LeaderIdIs(msg_data) => {
                self.logger.info("Received LeaderIdIs message");
                // Informar al CoordinatorManager sobre el nuevo líder

                if let Some(leader_addr) = self.ring_nodes.get(&msg_data.leader_id) {
                    self.current_coordinator = Some(*leader_addr);
                } else {
                    self.logger.info(format!(
                        "Leader ID {} not found in ring nodes.",
                        msg_data.leader_id
                    ));
                }

                if let Some(coordinator_manager) = &self.coordinator_manager {
                    coordinator_manager.do_send(msg_data);
                } else {
                    self.logger.info("CoordinatorManager not initialized yet.");
                }
            }
            NetworkMessage::RegisterUser(msg_data) => {
                let user_id = msg_data.user_id.clone();
                if let Some(communicator) = self.communicators.get(&msg_data.origin_addr) {
                    match communicator.peer_type {
                        PeerType::ClientType => {
                            let storage = self.storage.clone();
                            let client_id_clone = user_id.clone();
                            let logger = self.logger.clone();
                            ctx.spawn(
                                async move {
                                    match storage
                                        .as_ref()
                                        .unwrap()
                                        .send(GetClient {
                                            client_id: client_id_clone.clone(),
                                        })
                                        .await
                                    {
                                        Ok(user_dto_opt) => {
                                            if let Some(client_dto) = user_dto_opt {
                                                NetworkMessage::RecoveredInfo(UserDTO::Client(
                                                    client_dto,
                                                ))
                                            } else {
                                                storage.as_ref().unwrap().do_send(AddClient {
                                                    client: ClientDTO {
                                                        client_position: msg_data.position,
                                                        client_id: client_id_clone.clone(),
                                                        client_order: None,
                                                        time_stamp: std::time::SystemTime::now(),
                                                    },
                                                });
                                                NetworkMessage::NoRecoveredInfo
                                            }
                                        }
                                        Err(e) => {
                                            logger.error(format!(
                                                "Error retrieving client info: {}",
                                                e
                                            ));
                                            storage.as_ref().unwrap().do_send(AddClient {
                                                client: ClientDTO {
                                                    client_position: msg_data.position,
                                                    client_id: client_id_clone.clone(),
                                                    client_order: None,
                                                    time_stamp: std::time::SystemTime::now(),
                                                },
                                            });
                                            NetworkMessage::NoRecoveredInfo
                                        }
                                    }
                                }
                                .into_actor(self)
                                .map(
                                    move |network_message, actor, _ctx| {
                                        actor
                                            .send_network_message(user_id.clone(), network_message);
                                    },
                                ),
                            );
                        }
                        PeerType::RestaurantType => {
                            let storage = self.storage.clone();
                            let restaurant_id_clone = user_id.clone();
                            let logger = self.logger.clone();
                            ctx.spawn(
                                async move {
                                    match storage
                                        .as_ref()
                                        .unwrap()
                                        .send(GetRestaurant {
                                            restaurant_id: restaurant_id_clone.clone(),
                                        })
                                        .await
                                    {
                                        Ok(user_dto_opt) => {
                                            if let Some(restaurant_dto) = user_dto_opt {
                                                NetworkMessage::RecoveredInfo(UserDTO::Restaurant(
                                                    restaurant_dto,
                                                ))
                                            } else {
                                                storage.as_ref().unwrap().do_send(AddRestaurant {
                                                    restaurant: RestaurantDTO {
                                                        restaurant_position: msg_data.position,
                                                        restaurant_id: restaurant_id_clone.clone(),
                                                        authorized_orders: HashSet::new(),
                                                        pending_orders: HashSet::new(),
                                                        time_stamp: std::time::SystemTime::now(),
                                                    },
                                                });
                                                NetworkMessage::NoRecoveredInfo
                                            }
                                        }
                                        Err(e) => {
                                            logger.error(format!(
                                                "Error retrieving restaurant info: {}",
                                                e
                                            ));
                                            storage.as_ref().unwrap().do_send(AddRestaurant {
                                                restaurant: RestaurantDTO {
                                                    restaurant_position: msg_data.position,
                                                    restaurant_id: restaurant_id_clone.clone(),
                                                    authorized_orders: HashSet::new(),
                                                    pending_orders: HashSet::new(),
                                                    time_stamp: std::time::SystemTime::now(),
                                                },
                                            });
                                            NetworkMessage::NoRecoveredInfo
                                        }
                                    }
                                }
                                .into_actor(self)
                                .map(
                                    move |network_message, actor, _ctx| {
                                        actor
                                            .send_network_message(user_id.clone(), network_message);
                                    },
                                ),
                            );
                        }
                        PeerType::DeliveryType => {
                            let storage = self.storage.clone();
                            let delivery_id_clone = user_id.clone();
                            let logger = self.logger.clone();
                            ctx.spawn(
                                async move {
                                    match storage
                                        .as_ref()
                                        .unwrap()
                                        .send(GetDelivery {
                                            delivery_id: delivery_id_clone.clone(),
                                        })
                                        .await
                                    {
                                        Ok(user_dto_opt) => {
                                            if let Some(delivery_dto) = user_dto_opt {
                                                let delivery_dto = DeliveryDTO {
                                                    delivery_position: msg_data.position,
                                                    delivery_id: delivery_id_clone.clone(),
                                                    current_client_id: delivery_dto
                                                        .current_client_id,
                                                    current_order: delivery_dto.current_order,
                                                    status: delivery_dto.status,
                                                    time_stamp: std::time::SystemTime::now(),
                                                };
                                                storage.as_ref().unwrap().do_send(AddDelivery {
                                                    delivery: delivery_dto.clone(),
                                                });
                                                NetworkMessage::RecoveredInfo(UserDTO::Delivery(
                                                    delivery_dto,
                                                ))
                                            } else {
                                                storage.as_ref().unwrap().do_send(AddDelivery {
                                                    delivery: DeliveryDTO {
                                                        delivery_position: msg_data.position,
                                                        delivery_id: delivery_id_clone.clone(),
                                                        current_client_id: None,
                                                        current_order: None,
                                                        status: DeliveryStatus::Available,
                                                        time_stamp: std::time::SystemTime::now(),
                                                    },
                                                });
                                                NetworkMessage::NoRecoveredInfo
                                            }
                                        }
                                        Err(e) => {
                                            logger.error(format!(
                                                "Error retrieving delivery info: {}",
                                                e
                                            ));
                                            if let Some(storage_addr) = storage.as_ref() {
                                                storage_addr.do_send(AddDelivery {
                                                    delivery: DeliveryDTO {
                                                        delivery_position: msg_data.position,
                                                        delivery_id: delivery_id_clone.clone(),
                                                        current_client_id: None,
                                                        current_order: None,
                                                        status: DeliveryStatus::Available,
                                                        time_stamp: std::time::SystemTime::now(),
                                                    },
                                                });
                                            }

                                            NetworkMessage::NoRecoveredInfo
                                        }
                                    }
                                }
                                .into_actor(self)
                                .map(
                                    move |network_message, actor, _ctx| {
                                        actor
                                            .send_network_message(user_id.clone(), network_message);
                                    },
                                ),
                            );
                        }
                        _ => {
                            self.logger.info(format!(
                                "Received RegisterUser from non-client type: {:?}",
                                communicator.peer_type
                            ));
                        }
                    }
                } else {
                    self.logger.info(format!(
                        "Communicator not found for {}",
                        msg_data.origin_addr
                    ));
                }

                // TODO: Intentar recuperar información del usuario desde storage
                // self.send_network_message(client_id, NetworkMessage::NoRecoveredInfo);
            }

            // Client messages
            NetworkMessage::RequestThisOrder(msg_data) => {
                if let Some(order_service) = &self.order_service {
                    order_service.do_send(msg_data);
                } else {
                    self.logger.info("OrderService not initialized yet.");
                }
            }
            NetworkMessage::AuthorizationResult(_msg_data) => {
                self.logger
                    .info("Received AuthorizationResult message, not implemented yet");
            }
            NetworkMessage::NotifyOrderUpdated(_msg_data) => {
                self.logger
                    .info("Received NotifyOrderUpdated message, not implemented yet");
            }
            NetworkMessage::OrderFinalized(_msg_data) => {
                self.logger
                    .info("Received OrderFinalized message, not implemented yet");
            }
            NetworkMessage::RequestNearbyRestaurants(msg_data) => {
                if let Some(service) = &self.nearby_restaurant_service {
                    service.do_send(msg_data);
                } else {
                    self.logger
                        .info("NearbyRestaurantsService not initialized yet.");
                }
            }

            // Delivery messages
            NetworkMessage::IAmAvailable(_msg_data) => {
                self.logger
                    .info("Received IAmAvailable message, not implemented yet");
            }
            NetworkMessage::AcceptedOrder(msg_data) => {
                self.logger
                    .info("Received AcceptOrder message, not implemented yet");
                self.handle_order_accepted(msg_data.order.order_id, ctx);
                if let Some(order_service) = &self.order_service {
                    order_service.do_send(msg_data);
                } else {
                    self.logger.info("OrderService not initialized yet.");
                }
            }
            NetworkMessage::OrderDelivered(msg_data) => {
                if let Some(order_service) = &self.order_service {
                    order_service.do_send(UpdateOrderStatus {
                        order: msg_data.order.clone(),
                    });
                } else {
                    self.logger.info("OrderService not initialized yet.");
                }
            }
            NetworkMessage::IAmDelivering(msg_data) => {
                if let Some(order_service) = &self.order_service {
                    order_service.do_send(UpdateOrderStatus {
                        order: msg_data.order.clone(),
                    });
                } else {
                    self.logger.info("OrderService not initialized yet.");
                }
            }

            // Restaurant messages
            NetworkMessage::UpdateOrderStatus(msg_data) => {
                if let Some(order_service) = &self.order_service {
                    order_service.do_send(msg_data);
                } else {
                    self.logger.info("OrderService not initialized yet.");
                }
            }
            NetworkMessage::RequestNearbyDelivery(msg_data) => {
                if let Some(service) = &self.nearby_delivery_service {
                    service.do_send(msg_data);
                } else {
                    self.logger
                        .warn("NearbyDeliveryService not initialized yet.");
                }
            }
            NetworkMessage::DeliverThisOrder(msg_data) => {
                if let Some(order_service) = &self.order_service {
                    order_service.do_send(msg_data);
                } else {
                    self.logger.info("OrderService not initialized yet.");
                }
            }
            NetworkMessage::DeliveryAccepted(msg_data) => {
                self.logger
                    .info("Received DeliveryAccepted message, not implemented yet");
                if let Some(order_service) = &self.order_service {
                    order_service.do_send(msg_data);
                } else {
                    self.logger.info("OrderService not initialized yet.");
                }
            }

            // CoordinatorManager messages
            NetworkMessage::RequestNewStorageUpdates(msg_data) => {
                self.logger
                    .info("Received RequestNewStorageUpdates message");
                if let Some(coordinator_manager) = &self.coordinator_manager {
                    coordinator_manager.do_send(msg_data);
                } else {
                    self.logger.info("CoordinatorManager not initialized yet.");
                }
            }
            NetworkMessage::StorageUpdates(msg_data) => {
                self.logger.info("Received StorageUpdates message");
                if let Some(coordinator_manager) = &self.coordinator_manager {
                    coordinator_manager.do_send(msg_data);
                } else {
                    self.logger.info("CoordinatorManager not initialized yet.");
                }
            }
            NetworkMessage::RequestAllStorage(msg_data) => {
                self.logger.info("Received RequestAllStorage message");
                if let Some(coordinator_manager) = &self.coordinator_manager {
                    coordinator_manager.do_send(msg_data);
                } else {
                    self.logger.info("CoordinatorManager not initialized yet.");
                }
            }
            NetworkMessage::StorageSnapshot(msg_data) => {
                self.logger.info("Received StorageSnapshot message");
                if let Some(coordinator_manager) = &self.coordinator_manager {
                    coordinator_manager.do_send(msg_data);
                } else {
                    self.logger.info("CoordinatorManager not initialized yet.");
                }
            }
            NetworkMessage::RecoverStorageOperations(_msg_data) => {
                self.logger
                    .info("Received RecoverStorageOperations message");
            }
            NetworkMessage::LeaderElection(msg) => {
                self.logger.info(format!(
                    "Received LeaderElection message from {} with candidates {:?}",
                    msg.initiator, msg.candidates
                ));
                if let Some(coordinator_manager) = &self.coordinator_manager {
                    coordinator_manager.do_send(msg);
                } else {
                    self.logger.info("CoordinatorManager not initialized yet.");
                }
            }

            NetworkMessage::Ping(msg_data) => {
                self.logger.info("Received Ping message");
                if let Some(coordinator_manager) = &self.coordinator_manager {
                    coordinator_manager.do_send(msg_data);
                } else {
                    self.logger.info("CoordinatorManager not initialized yet.");
                }
            }

            NetworkMessage::Pong(msg_data) => {
                self.logger.info("Received Pong message");
                if let Some(coordinator_manager) = &self.coordinator_manager {
                    coordinator_manager.do_send(msg_data);
                } else {
                    self.logger.info("CoordinatorManager not initialized yet.");
                }
            }

            NetworkMessage::ConnectionClosed(msg_data) => {
                self.logger
                    .info(format!("Connection closed for {}", msg_data.remote_addr));
                let remote_addr = msg_data.remote_addr;
                // Si el remote_addr está en self.communicators, lo eliminamos
                if let Some(communicator) = self.communicators.get(&remote_addr) {
                    self.communicators.remove(&remote_addr);
                    self.user_addresses.remove_by_key(&remote_addr);

                    self.logger
                        .info(format!("Removed communicator for {}", remote_addr));
                } else {
                    if let Some(coordinator_manager) = &self.coordinator_manager {
                        // si el servidor que se desconectó era el lider actual, ponemos a nuestro lider como None
                        if self.current_coordinator == Some(remote_addr) {
                            self.current_coordinator = None;
                            self.logger
                                .info("Current coordinator set to None due to disconnection.");
                        }
                        // Enviamos el mensaje al CoordinatorManager para que maneje la desconexión

                        coordinator_manager.do_send(msg_data);
                    } else {
                        self.logger.info("CoordinatorManager not initialized yet.");
                    }
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
