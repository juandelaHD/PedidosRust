use actix::prelude::*;
use common::messages::shared_messages::{NetworkMessage, WhoIsLeader, LeaderIs, StartRunning, NewLeaderConnection};
use common::messages::client_messages::*;
use std::collections::HashMap;
use std::fmt::format;
use std::net::SocketAddr;
use tokio::net::{TcpStream};
use crate::client_actors::ui_handler::UIHandler;

use common::logger::Logger;
use common::network::communicator::Communicator;
use common::network::peer_types::PeerType;
use common::types::dtos::OrderDTO;
use common::network::connections::{connect, connect_to_all};
use common::types::order_status::OrderStatus;

use crate::messages::internal_messages::SendThisOrder;


pub struct Client {
    /// Vector de direcciones de servidores
    pub servers: Vec<SocketAddr>,
    /// Identificador único del comensal.
    pub client_id: String,
    /// Posición actual del cliente en coordenadas 2D.
    pub position: (f32, f32),
    /// Estado actual del pedido (si hay uno en curso).
    pub order_status: Option<OrderStatus>,
    /// Restaurante elegido para el pedido.
    pub selected_restaurant: Option<String>,
    /// ID del pedido actual.
    pub order_id: Option<u64>,
    /// Canal de envío hacia el actor `UIHandler`.
    pub ui_handler: Option<Addr<UIHandler>>,
    /// Comunicador asociado al `Server`.
    pub actual_communicator_addr: Option<SocketAddr>,
    pub communicators: Option<HashMap<SocketAddr,Communicator<Client>>>,
    pub pending_streams: HashMap<SocketAddr, TcpStream>, // Guarda el stream hasta que arranque
    pub my_socket_addr: SocketAddr,
    pub logger: Logger,
}

impl Client {
    pub async fn new(servers: Vec<SocketAddr>, id: String, position: (f32, f32)) -> Self {
        let pending_streams: HashMap<SocketAddr, TcpStream> = connect_to_all(servers.clone()).await;

        let logger = Logger::new(format!("Client {}", &id));

        if pending_streams.is_empty() {
            panic!("Unable to connect to any server.");
        }

        // Tomamos la primera conexión como referencia para my_socket_addr
        let my_socket_addr = pending_streams
            .values()
            .next()
            .expect("No stream available")
            .local_addr()
            .expect("Failed to get local socket addr");

        Self {
            servers,
            client_id: id,
            position,
            order_status: None, // Inicialmente no hay pedido
            selected_restaurant: None, // Inicialmente no hay restaurante seleccionado
            order_id: None, // Inicialmente no hay ID de pedido
            ui_handler: None, // Inicialmente no hay UIHandler
            actual_communicator_addr: None, // Podés usarlo para el líder actual
            communicators: Some(HashMap::new()), // Inicializamos el hashmap vacío para los communicators
            pending_streams: pending_streams,
            my_socket_addr,
            logger,
        }
    }

    pub fn send_network_message(&self, message: NetworkMessage) {
        if let (Some(communicators_map), Some(addr)) = (&self.communicators, self.actual_communicator_addr) {
            if let Some(communicator) = communicators_map.get(&addr) {
                if let Some(sender) = &communicator.sender{
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


impl Actor for Client {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let mut communicators_map = HashMap::new();

        for (addr, stream) in self.pending_streams.drain() {
            let communicator = Communicator::new(
                stream,
                ctx.address(),
                PeerType::ClientType,
            );
            communicators_map.insert(addr, communicator);
            self.actual_communicator_addr = Some(addr);
            self.logger.info(format!("Communicator started for {}", addr));
        }
        self.communicators = Some(communicators_map);
        let ui_handler = UIHandler::new(ctx.address(), self.logger.clone());
        self.ui_handler = Some(ui_handler.start());
    }
}

impl Handler<StartRunning> for Client {
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


impl Handler<NewLeaderConnection> for Client {
    type Result = ();

    fn handle(&mut self, msg: NewLeaderConnection, ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!("Creating new Communicator for leader {}", msg.addr));

        let communicator = Communicator::new(
            msg.stream,
            ctx.address(),
            PeerType::ClientType,
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


impl Handler<LeaderIs> for Client {
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

impl Handler<SendThisOrder> for Client {
    type Result = ();

    fn handle(&mut self, msg: SendThisOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!("Sending order to restaurant {}: {}", msg.selected_restaurant, msg.selected_dish));
        
        let order = OrderDTO {
            order_id: self.order_id.unwrap_or(0), // Si no hay ID, usar 0 o generar uno nuevo
            client_id: self.client_id.clone(),
            restaurant_id: msg.selected_restaurant,
            dish_name: msg.selected_dish,
            status: OrderStatus::Pending, // Estado inicial del pedido
            delivery_id: None, // No hay delivery asignado aún
            time_stamp: std::time::SystemTime::now(), // Marca de tiempo actual
            client_position: self.position, // Posición del cliente
        };

        // Enviar el pedido al servidor
        let network_message = NetworkMessage::RequestThisOrder(
            RequestThisOrder {
                order
            }
        );
        self.send_network_message(network_message);
    }
}

impl Handler<NetworkMessage> for Client {
    type Result = ();
    fn handle(&mut self, msg: NetworkMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NetworkMessage::WhoIsLeader(_msg_data) => {
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
            NetworkMessage::RequestNearbyRestaurants(_msg_data) => {
                self.logger.error("Received a WhoIsLeader message, handle not implemented");
                
                // Aquí podrías enviar una respuesta o manejar la solicitud de restaurantes cercanos  
            }
            NetworkMessage::RequestThisOrder(_msg_data) => {
                self.logger.error("Received a RequestThisOrder message, handle not implemented");
            
            }
        }
    }
}


