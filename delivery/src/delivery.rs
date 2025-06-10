use actix::prelude::*;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use common::logger::Logger;
use common::messages::shared_messages::*;
use common::network::connections::{connect, connect_to_all};
use common::network::communicator::Communicator;
use common::network::peer_types::PeerType;
use common::types::dtos::OrderDTO;
use common::types::delivery_status::DeliveryStatus;
use std::collections::HashMap;

use common::types::dtos::UserDTO;



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
    pub communicators: Option<HashMap<SocketAddr,Communicator<Delivery>>>,
    pub pending_streams: HashMap<SocketAddr, TcpStream>, // Guarda los streams hasta que arranque
    pub my_socket_addr: SocketAddr,
    pub logger: Logger,
}

impl Delivery {
    pub async fn new(servers: Vec<SocketAddr>, id: String, position: (f32, f32), probability: f32) -> Self {
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
            status: DeliveryStatus::Available,
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
        if let (Some(communicators_map), Some(addr)) = (&self.communicators, self.actual_communicator_addr) {
            if let Some(communicator) = communicators_map.get(&addr) {
                if let Some(sender) = communicator.sender.as_ref() {
                    sender.do_send(message);
                } else {
                    self.logger.error("Sender not initialized in communicator");
                }
            } else {
                self.logger.error(&format!("Communicator not found for addr {}", addr));
            }
        } else {
            self.logger.error("Communicators map or actual_communicator_addr not initialized");
        }
    }
}


impl Actor for Delivery {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let mut communicators_map = HashMap::new();

        for (addr, stream) in self.pending_streams.drain() {
            let communicator = Communicator::new(
                stream,
                ctx.address(),
                PeerType::DeliveryType,
            );
            communicators_map.insert(addr, communicator);
            self.actual_communicator_addr = Some(addr);
            self.logger.info(format!("Communicator started for {}", addr));
        }
        self.communicators = Some(communicators_map);
    }
}

impl Handler<StartRunning> for Delivery {
    type Result = ();

    fn handle(&mut self, _msg: StartRunning, _ctx: &mut Self::Context) {
        self.logger.info("Starting delivery...");
        self.send_network_message(
            NetworkMessage::WhoIsLeader(
                WhoIsLeader {
                    origin_addr: self.my_socket_addr
                }
            ));
    }
}

impl Handler<NewLeaderConnection> for Delivery {
    type Result = ();

    fn handle(&mut self, msg: NewLeaderConnection, ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!("Creating new Communicator for leader {}", msg.addr));

        let communicator = Communicator::new(
            msg.stream,
            ctx.address(),
            PeerType::DeliveryType,
        );

        if let Some(communicators_map) = &mut self.communicators {
            communicators_map.insert(msg.addr, communicator);
            self.actual_communicator_addr = Some(msg.addr);
            self.logger.info(format!("New leader Communicator established at {}", msg.addr));
        } else {
            self.logger.error("Communicators map not initialized when creating new leader Communicator");
        }
    }
}


impl Handler<LeaderIs> for Delivery {
    type Result = ();

    fn handle(&mut self, msg: LeaderIs, ctx: &mut Self::Context) -> Self::Result {
        let leader_addr = msg.coord_addr;

        self.logger.info(format!("Received LeaderIs message with addr: {}", leader_addr));

        // Verificar si ya tenemos un Communicator para el nuevo líder
        if let Some(communicators_map) = &mut self.communicators {
            if communicators_map.contains_key(&leader_addr) {
                // Ya tenemos conexión con el líder, actualizar
                self.actual_communicator_addr = Some(leader_addr);
                self.logger.info(format!("Updated actual_communicator_addr to {}", leader_addr));
            } else {
                // No existe aún, tenemos que crearla (async dentro de sync handler usando spawn)
                let client_addr = ctx.address();
                let logger_clone = self.logger.clone();

                actix::spawn(async move {
                    logger_clone.info(format!("Connecting to new leader at {}", leader_addr));
                    if let Some(stream) = connect(leader_addr).await {
                        logger_clone.info(format!("Successfully connected to leader at {}", leader_addr));

                        client_addr.do_send(NewLeaderConnection { addr: leader_addr, stream });
                    } else {
                        logger_clone.error(format!("Failed to connect to new leader at {}", leader_addr));
                    }
                });
            }
            println!("Sending msg RegisterUser with ID: {}", self.delivery_id);
            self.send_network_message(
                NetworkMessage::RegisterUser(
                    RegisterUser {
                        origin_addr: self.my_socket_addr,
                        user_id: self.delivery_id.clone(),
                    }
                )
            );
            self.logger.info(format!("Sending msg register user with ID: {}", self.delivery_id));

        } else {
            self.logger.error("Communicators map not initialized");
        }
    }

}






impl Handler<NetworkMessage> for Delivery {
    type Result = ();
    fn handle(&mut self, msg: NetworkMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            // All Users messages
            NetworkMessage::WhoIsLeader(_msg_data) => {
                self.logger.error("Received a WhoIsLeader message, handle not implemented");
            }
            NetworkMessage::LeaderIs(msg_data) => {
                ctx.address().do_send(msg_data)
            }
            NetworkMessage::RegisterUser(_msg_data) => {
                self.logger.info("Received RegisterUser message, not implemented yet");
            }
            NetworkMessage::RecoveredInfo(user_dto_opt) => {
                match user_dto_opt {
                    Some(user_dto) => match user_dto {
                        UserDTO::Delivery(delivery_dto) => {
                            if delivery_dto.delivery_id == self.delivery_id {
                                self.logger.info(format!(
                                    "Recovered info for Delivery ID={}, updating local state...",
                                    delivery_dto.delivery_id
                                ));

                                self.position = delivery_dto.delivery_position;
                                self.status = delivery_dto.status;
                                self.current_order = delivery_dto.current_order.clone();

                                self.logger.info(format!(
                                    "Updated position=({:?}), status={:?}, current_order_id={:?}",
                                    self.position,
                                    self.status,
                                    delivery_dto.current_order.as_ref().map(|o| o.order_id),
                                ));
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
                    None => {
                        self.logger.info("No recovered info found for this Delivery.");
                    }
                }
            }


            // Client messages
            NetworkMessage::NearbyRestaurants(msg_data) => {
                self.logger.info(format!(
                    "Received NearbyRestaurants message with {} restaurants",
                    msg_data.restaurants.len()));
            }
            NetworkMessage::AuthorizationResult(_msg_data) => {
                self.logger.info("Received AuthorizationResult message, not implemented yet");
            }
            NetworkMessage::NotifyOrderUpdated(_msg_data) => {
                self.logger.info("Received NotifyOrderUpdated message, not implemented yet");
            }
            NetworkMessage::OrderFinalized(_msg_data) => {
                self.logger.info("Received OrderFinalized message, not implemented yet");
            }
            NetworkMessage::RequestNearbyRestaurants(_msg_data) => {
                self.logger.info("Received RequestNearbyRestaurants message");
            }
            NetworkMessage::RequestThisOrder(_msg_data) => {
                self.logger.info("Received RequestThisOrder message");
            }

            // Delivery messages
            NetworkMessage::IAmAvailable(_msg_data) => {
                self.logger.info("Received IAmAvailable message, not implemented yet");
            }
            NetworkMessage::AcceptOrder(_msg_data) => {
                self.logger.info("Received AcceptOrder message, not implemented yet");
            }
            NetworkMessage::OrderDelivered(_msg_data) => {
                self.logger.info("Received OrderDelivered message, not implemented yet");
            }

            
            // Restaurant messages
            NetworkMessage::UpdateOrderStatus(_msg_data) => {
                self.logger.info("Received UpdateOrderStatus message, not implemented yet");
            }
            NetworkMessage::CancelOrder(_msg_data) => {
                self.logger.info("Received CancelOrder message, not implemented yet");
            }
            NetworkMessage::OrderIsPreparing(_msg_data) => {
                self.logger.info("Received OrderIsPreparing message, not implemented yet");
            }
            NetworkMessage::RequestDelivery(_msg_data) => {
                self.logger.info("Received RequestDelivery message, not implemented yet");
            }
            NetworkMessage::DeliverThisOrder(_msg_data) => {
                self.logger.info("Received DeliverThisOrder message, not implemented yet");
            }

            // Coordinator messages
            NetworkMessage::RequestNewStorageUpdates(_msg_data) => {
                self.logger.info("Received RequestNewStorageUpdates message");
            }
            NetworkMessage::StorageUpdates(_msg_data) => {
                self.logger.info("Received StorageUpdates message");
            }
            NetworkMessage::RequestAllStorage(_msg_data) => {
                self.logger.info("Received RequestAllStorage message");
            }
            NetworkMessage::RecoverStorageOperations(_msg_data) => {
                self.logger.info("Received RecoverStorageOperations message");
            }
            NetworkMessage::LeaderElection(_msg_data) => {
                self.logger.info("Received LeaderElection message");
            }
        }
    }
}
