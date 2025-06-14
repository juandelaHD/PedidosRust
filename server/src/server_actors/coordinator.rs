use crate::messages::internal_messages::RegisterConnection;
use crate::server_actors::coordinator_manager::CoordinatorManager;
use actix::prelude::*;
use common::bimap::BiMap;
use common::constants::BASE_PORT;
use common::logger::Logger;
use common::messages::coordinator_messages::*;
use common::messages::shared_messages::*;
use common::network::communicator::Communicator;
use common::types::restaurant_info;
use std::{collections::HashMap, net::SocketAddr};
use common::network::connections::{connect_to_all};
use tokio::net::TcpStream;
use common::network::peer_types::PeerType;
use crate::messages::internal_messages::*;

#[derive(Debug)]
pub struct Coordinator {
    pub id: String,
    /// Direcciones de todos los nodos en el anillo.
    pub ring_nodes: Vec<SocketAddr>,
    /// Dirección de este coordinator.
    pub my_addr: SocketAddr,
    /// Coordinador actual.
    pub current_coordinator: Option<SocketAddr>,
    /// Estado de los pedidos en curso.
    //   pub active_orders: HashSet<u64>, // TODO: Ver si se puede sacar
    /// Comunicador con el PaymentGateway
    // pub payment_communicator: Communicator,
    /// Diccionario de direcciones de usuarios con sus correspondientes IDs.
    pub user_addresses: BiMap<SocketAddr, String>, // ID -> REMOTA
    /// Mapa de comunicadores de clientes conectados.
    pub communicators: HashMap<SocketAddr, Communicator<Coordinator>>, // REMOTA -> Comunicador
    /// Canal de envío hacia el actor `Storage`.
    // pub storage: Addr<Storage>,
    /// Canal de envío hacia el actor `Reaper`.
    // pub reaper: Addr<Reaper>,
    /// Servicio de órdenes.
    // pub order_service: Addr<OrderService>,
    /// Servicio de restaurantes cercanos.
    // pub nearby_restaurant_service: Addr<NearbyRestaurantService>,
    // Servicio de deliverys cercanos.
    // pub nearby_delivery_service: Addr<NearbyDeliveryService>,
    pub logger: Logger,
    pub coordinator_manager: Option<Addr<CoordinatorManager>>,
    pub pending_streams: HashMap<SocketAddr, TcpStream>,
}

impl Coordinator {
    pub async fn new(
        srv_addr: SocketAddr,
        ring_nodes: Vec<SocketAddr>,
    ) -> Self {
        // Inicializar el coordinador con la dirección del servidor y los nodos del anillo
        // y un logger compartido.
        let pending_streams: HashMap<SocketAddr, TcpStream> =
            connect_to_all(ring_nodes.clone(), PeerType::CoordinatorType).await;

        if pending_streams.is_empty() {
            println!("No connections established.");
        }

        Self {
            id: format!("server_{}", srv_addr.port() - BASE_PORT),
            ring_nodes,
            my_addr: srv_addr,
            current_coordinator: None,
            user_addresses: BiMap::new(),
            logger: Logger::new("COORDINATOR"),
            coordinator_manager: None,
            communicators: HashMap::new(),
            pending_streams,
        }
    }

    
}

impl Actor for Coordinator {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let coordinator_manager = CoordinatorManager::new(
            self.id.clone(),
            self.my_addr,
            self.ring_nodes.clone(),
            ctx.address(),
        );
        self.coordinator_manager = Some(coordinator_manager.start());
        self.logger.info("Coordinator started.");

        for (addr, stream) in self.pending_streams.drain() {
            let communicator = Communicator::new(stream, ctx.address(), PeerType::CoordinatorType);
            // le paso los coordinadores que hay al CoordinatorManager
            if let Some(coordinator_manager) = &self.coordinator_manager {
                coordinator_manager.do_send(RegisterConnectionManager {
                    remote_addr: addr,
                    communicator: communicator,
                });
            } else {
                self.logger
                    .info("CoordinatorManager not initialized yet, cannot register connection.");
            }
        }

        self.logger.info("Communicators initialized.");

        // Enviar un startRunning al CoordinatorManager
        if let Some(coordinator_manager) = &self.coordinator_manager {
            coordinator_manager.do_send(StartRunning);
        } else {
            self.logger
                .info("CoordinatorManager not initialized yet, cannot start running.");
        }
    }
}


impl Handler<RegisterConnectionManager> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: RegisterConnectionManager, _ctx: &mut Context<Self>) {
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

impl Handler<RegisterConnection> for Coordinator {
    type Result = ();
    fn handle(&mut self, msg: RegisterConnection, _ctx: &mut Self::Context) -> Self::Result {
        // Registrar la conexión del cliente
        self.communicators.insert(msg.client_addr, msg.communicator);

        // TODO: El valor debe ser el ID del cliente (el nombre)
        self.user_addresses
            .insert(msg.client_addr,  "UNKNOWN_USER".to_string());
        self.logger
            .info(format!("Registered connection from {} ", msg.client_addr));
    }
}

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
            self.user_addresses.insert(user_address,  msg.user_id.clone());
            self.logger.info(format!(
                "User ID for {} is {}",
                user_address, msg.user_id.clone()
            ));

        } else {
            self.logger.info(format!(
                "No user ID found for {}. Adding as UNKNOWN_USER.",
                user_address
            ));
            // Si no está, lo actualizamos o lo agregamos
            self.user_addresses.insert(user_address, msg.user_id.clone());
        }

        //  Si hay un coordinador actual, se lo notificamos al cliente
        if let Some(addr) = self.current_coordinator {
            if let Some(sender) = &self.communicators[&msg.origin_addr].sender {
                sender.do_send(NetworkMessage::LeaderIs(LeaderIs { coord_addr: (addr) }));
            } else {
                self.logger
                    .info(format!("No sender found for {}", msg.origin_addr));
            }
        } else {
            // Si no hay coordinador actual, le preguntamos al CoordinatorManager
            self.logger
                .info("No current coordinator available. Check Server Election implementation.");
            // TODO: delete this when election is ready
            if let Some(sender) = &self.communicators[&msg.origin_addr].sender {
                sender.do_send(NetworkMessage::LeaderIs(LeaderIs {
                    coord_addr: (self.my_addr),
                }));
            } else {
                self.logger
                    .info(format!("No sender found for {}", msg.origin_addr));
            }
        }
    }
}

impl Handler<LeaderIs> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: LeaderIs, _ctx: &mut Self::Context) -> Self::Result {
        // Actualizar el coordinador actual
        self.current_coordinator = Some(msg.coord_addr);
        self.logger.info(format!(
            "Updated current coordinator to: {}",
            msg.coord_addr
        ));
    }
}

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

impl Handler<NetworkMessage> for Coordinator {
    type Result = ();
    fn handle(&mut self, msg: NetworkMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            // All Users messages
            NetworkMessage::WhoIsLeader(msg_data) => {
                if self.current_coordinator.is_none() {
                    ctx.address().do_send(RetryLater {
                        origin_addr: msg_data.origin_addr,
                    });
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
                    // Aquí podrías responder directamente o ignorar
                    ctx.address().do_send(msg_data)
                }
            }
            NetworkMessage::LeaderIs(msg_data) => {
                self.logger.info("Received LeaderIs message");
                // Informar al CoordinatorManager sobre el nuevo líder
                if let Some(coordinator_manager) = &self.coordinator_manager {
                    coordinator_manager.do_send(msg_data);
                } else {
                    self.logger.info("CoordinatorManager not initialized yet.");
                }
            }
            NetworkMessage::RegisterUser(msg_data) => {
                self.logger
                    .info("Received RegisterUser message, not implemented yet, sending dummy response");
                // TODO: Implement user registration logic to return user info
                let client_id = msg_data.user_id.clone();
                let client_to_send = self.user_addresses.get_by_value(&client_id).cloned();
                if let Some(client_addr) = client_to_send {
                    if let Some(sender) = &self.communicators[&client_addr].sender {
                        // TODO: Intentar recuperar información del usuario desde storage
                        self.logger.warn(format!(
                            "Sending NoRecoveredInfo to {} for client ID {}, due to no storage implementation",
                            client_addr, client_id
                        ));
                        sender.do_send(NetworkMessage::NoRecoveredInfo);
                    } else {
                        self.logger
                            .info(format!("No sender found for {} to send information", client_addr));
                    }
                } else {
                    self.logger.info(format!(
                        "Client ID {} not found in user_addresses",
                        client_id
                    ));
                }
            }

            // Client messages
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
                self.logger
                    .info("Received RequestNearbyRestaurants message");
                
                let restaurant_info_dummy = restaurant_info::RestaurantInfo {
                    id: "dummy_restaurant".to_string(),
                    position: (2.0, 0.0),
                };
                let restaurants_dummy = NearbyRestaurants {
                    restaurants: vec![restaurant_info_dummy],
                };
                let client_id = msg_data.client.client_id;
                let client_to_send = self.user_addresses.get_by_value(&client_id).cloned();
                if let Some(client_addr) = client_to_send {
                    if let Some(sender) = &self.communicators[&client_addr].sender {
                        sender.do_send(NetworkMessage::NearbyRestaurants(restaurants_dummy));
                    } else {
                        self.logger
                            .info(format!("No sender found for {}", client_addr));
                    }
                } else {
                    self.logger.info(format!(
                        "Client ID {} not found in user_addresses",
                        client_id
                    ));
                }
            }
            NetworkMessage::RequestThisOrder(_msg_data) => {
                self.logger.info("Received RequestThisOrder message");
            }

            // Delivery messages
            NetworkMessage::IAmAvailable(_msg_data) => {
                self.logger
                    .info("Received IAmAvailable message, not implemented yet");
            }
            NetworkMessage::AcceptOrder(_msg_data) => {
                self.logger
                    .info("Received AcceptOrder message, not implemented yet");
            }
            NetworkMessage::OrderDelivered(_msg_data) => {
                self.logger
                    .info("Received OrderDelivered message, not implemented yet");
            }

            // Restaurant messages
            NetworkMessage::UpdateOrderStatus(_msg_data) => {
                self.logger
                    .info("Received UpdateOrderStatus message, not implemented yet");
            }
            NetworkMessage::CancelOrder(_msg_data) => {
                self.logger
                    .info("Received CancelOrder message, not implemented yet");
            }
            NetworkMessage::OrderIsPreparing(_msg_data) => {
                self.logger
                    .info("Received OrderIsPreparing message, not implemented yet");
            }
            NetworkMessage::RequestDelivery(_msg_data) => {
                self.logger
                    .info("Received RequestDelivery message, not implemented yet");
            }
            NetworkMessage::DeliverThisOrder(_msg_data) => {
                self.logger
                    .info("Received DeliverThisOrder message, not implemented yet");
            }

            // CoordinatorManager messages
            NetworkMessage::RequestNewStorageUpdates(_msg_data) => {
                self.logger
                    .info("Received RequestNewStorageUpdates message");
            }
            NetworkMessage::StorageUpdates(_msg_data) => {
                self.logger.info("Received StorageUpdates message");
            }
            NetworkMessage::RequestAllStorage(_msg_data) => {
                self.logger.info("Received RequestAllStorage message");
            }
            NetworkMessage::RecoverStorageOperations(_msg_data) => {
                self.logger
                    .info("Received RecoverStorageOperations message");
            }
            NetworkMessage::LeaderElection(_msg_data) => {
                self.logger.info("Received LeaderElection message");
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
