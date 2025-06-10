use actix::prelude::*;
use common::messages::shared_messages::LeaderIs;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;

use common::logger::Logger;
use common::messages::shared_messages::NetworkMessage;
use common::messages::shared_messages::StartRunning;
use common::messages::shared_messages::WhoIsLeader;
use common::network::communicator::Communicator;
use common::network::peer_types::PeerType;
use common::network::tcp_receiver::TCPReceiver;
use common::network::tcp_sender::TCPSender;
use common::types::dtos::OrderDTO;

pub struct Client {
    /// Vector de direcciones de servidores
    pub servers: Vec<SocketAddr>,
    /// Identificador único del comensal.
    pub client_id: String,
    /// Posición actual del cliente en coordenadas 2D.
    pub position: (f32, f32),
    /// Estado actual del pedido (si hay uno en curso).
    // pub order_status: Option<OrderStatus>,
    /// Restaurante elegido para el pedido.
    // pub selected_restaurant: Option<String>,
    /// ID del pedido actual.
    // pub order_id: Option<u64>,
    /// Canal de envío hacia el actor `UIHandler`.
    // pub ui_handler: Addr<UIHandler>,
    /// Comunicador asociado al `Server`.
    pub communicator: Option<Communicator<Client>>,
    pub pending_stream: Option<TcpStream>, // Guarda el stream hasta que arranque
    pub my_socket_addr: SocketAddr,
    pub logger: Logger,
}

impl Client {
    pub async fn new(servers: Vec<SocketAddr>, id: String, position: (f32, f32)) -> Self {
        let tcp_stream: Option<TcpStream> = connect(servers.clone()).await;
        let logger = Logger::new(format!("Delivery {}", &id));
        if let Some(stream) = tcp_stream {
            let my_socket_addr = stream
                .local_addr()
                .expect("No se pudo obtener la dirección local del socket");
            Self {
                servers,
                client_id: id,
                position,
                communicator: None,
                pending_stream: Some(stream),
                my_socket_addr,
                logger,
            }
        } else {
            panic!("Unable to connect to any server.");
        }
    }
}

impl Actor for Client {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        if let Some(stream) = self.pending_stream.take() {
            // Ahora podés crear el Communicator, ya que tenés el ctx.address()
            self.communicator = Some(Communicator::new(
                stream,
                ctx.address(),
                PeerType::DeliveryType,
            ));
            self.logger.info("Communicator started");
        } else {
            self.logger.error("No stream available");
        }
    }
}

impl Handler<StartRunning> for Client {
    type Result = ();

    fn handle(&mut self, _msg: StartRunning, _ctx: &mut Self::Context) {
        self.logger.info("Starting client...");
        if let Some(communicator) = &self.communicator {
            if let Some(sender) = communicator.sender.as_ref() {
                sender.do_send(NetworkMessage::WhoIsLeader(WhoIsLeader {
                    origin_addr: (self.my_socket_addr),
                }));
            } else {
                self.logger.error("Sender not initialized in communicator");
            }
        } else {
            self.logger.error("Communicator not initialized");
        }
    }
}

impl Handler<NetworkMessage> for Client {
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


async fn connect(servers: Vec<SocketAddr>) -> Option<TcpStream> {
    // Aca deberia preguntarle sobre informacion si se reconectó?
    for addr in servers {
        if let Ok(stream) = TcpStream::connect(addr).await {
            return Some(stream);
        }
    }
    None
}

impl Handler<LeaderIs> for Client {
    type Result = ();

    fn handle(&mut self, msg: LeaderIs, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!("Received LeaderIs message with addr: {}", msg.coord_addr));
        
    }
}