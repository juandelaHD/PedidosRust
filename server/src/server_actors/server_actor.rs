use actix::prelude::*;
use common::logger::Logger;
use common::messages::shared_messages::LeaderIs;
use common::messages::shared_messages::NetworkMessage;
use common::messages::shared_messages::WhoIsLeader;
use common::network::communicator::Communicator;
use std::sync::Arc;
use std::{collections::HashMap, net::SocketAddr};

use crate::server_actors::coordinator_manager::CoordinatorManager;

use crate::messages::internal_messages::RegisterConnection;

#[derive(Debug)]
pub struct Coordinator {
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
    /// Diccionario de conexiones activas con clientes, restaurantes y deliverys.
    pub communicators: HashMap<SocketAddr, Communicator<Coordinator>>,
    /// Diccionario de direcciones de usuarios con sus correspondientes IDs.
    pub user_addresses: HashMap<SocketAddr, String>,
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
    pub logger: Arc<Logger>,
    pub coordinator_manager: Addr<CoordinatorManager>,
}

impl Actor for Coordinator {
    type Context = Context<Self>;
}

impl Coordinator {
    pub fn new(
        srv_addr: SocketAddr,
        ring_nodes: Vec<SocketAddr>,
        logger: Arc<Logger>,
    ) -> Addr<Self> {
        Coordinator::create(move |ctx| {
            let coordinator_manager = CoordinatorManager::new(
                srv_addr,
                ring_nodes.clone(),
                ctx.address(),
                logger.clone(),
            );

            Coordinator {
                my_addr: srv_addr,
                ring_nodes,
                current_coordinator: None,
                communicators: HashMap::new(),
                user_addresses: HashMap::new(),
                logger: logger.clone(),
                coordinator_manager,
            }
        })
    }
}

impl Handler<RegisterConnection> for Coordinator {
    type Result = ();
    fn handle(&mut self, msg: RegisterConnection, _ctx: &mut Self::Context) -> Self::Result {
        // Registrar la conexión del cliente
        self.communicators.insert(msg.client_addr, msg.communicator);

        // TODO: El valor debe ser el ID del cliente (el nombre)
        self.user_addresses
            .insert(msg.client_addr, msg.client_addr.to_string());
        self.logger
            .info(format!("Registered connection from {} ", msg.client_addr));
        // Aquí podrías enviar un mensaje de bienvenida o iniciar alguna lógica adicional
    }
}

impl Handler<WhoIsLeader> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: WhoIsLeader, _ctx: &mut Self::Context) -> Self::Result {
        // Enviar un mensaje al líder actual o iniciar una elección de líder
        if let Some(addr) = self.current_coordinator {
            if let Some(sender) = &self.communicators[&msg.origin_addr].sender {
                sender.do_send(NetworkMessage::LeaderIs(LeaderIs { coord_addr: (addr) }));
            } else {
                self.logger
                    .info(format!("No sender found for {}", msg.origin_addr));
            }
        } else {
            // TODO: check start election if addr is NONE
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

impl Handler<NetworkMessage> for Coordinator {
    type Result = ();
    fn handle(&mut self, msg: NetworkMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            // All Users messages
            NetworkMessage::WhoIsLeader(msg_data) => {
                self.logger.info("Received WhoIsLeader message");
                ctx.address().do_send(msg_data);
            }
            NetworkMessage::LeaderIs(_msg_data) => {
                self.logger.info("Received LeaderIs message with addr:");
            }
            NetworkMessage::RegisterUser(_msg_data) => {
                self.logger
                    .info("Received RegisterUser message, not implemented yet");
            }
            NetworkMessage::RecoveredInfo(_user_dto_opt) => {
                self.logger
                    .info("Received RegisterUser message, not implemented yet");
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
            NetworkMessage::RequestNearbyRestaurants(_msg_data) => {
                self.logger
                    .info("Received RequestNearbyRestaurants message");
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
