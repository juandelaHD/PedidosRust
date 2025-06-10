use actix::prelude::*;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;

use common::logger::Logger;
use common::messages::shared_messages::{NetworkMessage, WhoIsLeader, LeaderIs, StartRunning, NewLeaderConnection};
use common::network::connections::{connect, connect_to_all};
use common::network::communicator::Communicator;
use common::network::peer_types::PeerType;
use common::network::tcp_receiver::TCPReceiver;
use common::network::tcp_sender::TCPSender;
use common::types::dtos::OrderDTO;
use common::types::delivery_status::DeliveryStatus;
use std::collections::HashMap;

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
        self.logger.info("Starting client...");
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
        } else {
            self.logger.error("Communicators map not initialized");
        }
    }
}

impl Handler<NetworkMessage> for Delivery {
    type Result = ();
    fn handle(&mut self, msg: NetworkMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NetworkMessage::WhoIsLeader(msg_data) => {
                self.logger.error("Received a WhoIsLeader message, handle not implemented");
            }
            NetworkMessage::LeaderIs(msg_data) => {
                ctx.address().do_send(msg_data)
            }
            NetworkMessage::RequestNewStorageUpdates(msg_data) => {
                self.logger.info(
                    "Received RequestNewStorageUpdates message with start_index:",
                    
                );
            }
            NetworkMessage::StorageUpdates(msg_data) => {
                self.logger.info(
                    "Received StorageUpdates message with updates",
                    );
            }
            NetworkMessage::RequestAllStorage(msg_data) => {
                self.logger.info("Received RequestAllStorage message");
            }
            NetworkMessage::RecoverStorageOperations(msg_data) => {
                self.logger.info(
                    "Received RecoverStorageOperations message with {} recover msgs and {} log msgs",
                    
                );
            }
            NetworkMessage::LeaderElection(msg_data) => {
                self.logger.info(
                    "Received LeaderElection message with candidates",
                
                );
            }
        }
    }
}
