use actix::prelude::*;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;

use common::logger::Logger;
use common::messages::shared_messages::NetworkMessage;
use common::messages::shared_messages::StartRunning;
use common::network::communicator::Communicator;
use common::network::peer_types::PeerType;
use common::network::tcp_receiver::TCPReceiver;
use common::network::tcp_sender::TCPSender;
use common::types::delivery_status::ClientStatus;
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
    pub communicator: Communicator,
    pub logger: Logger,
}

impl Client {
    pub async fn new(servers: Vec<SocketAddr>, id: String, position: (f32, f32)) -> Self {
        let tcp_stream: Option<TcpStream> = connect(servers.clone()).await;
        let logger = Logger::new(format!("Client {}", &id));
        if let Some(stream) = tcp_stream {
            logger.info(format!("Connected to server at {:?}", stream.peer_addr()));
            Self {
                servers,
                client_id: id,
                position,
                //status,
                communicator: Communicator::new(stream, PeerType::ClientType), // aca falta agregarle el context del cliente
                logger,
            }
        } else {
            panic!("Unable to connect to any server.");
        }
    }
}

impl Actor for Client {
    type Context = Context<Self>;
}

impl Handler<StartRunning> for Delivery {
    type Result = ();

    fn handle(&mut self, _msg: StartRunning, ctx: &mut Self::Context) {
        // envia el who is leader
    }
}

impl Handler<NetworkMessage> for Delivery {
    type Result = ();

    fn handle(&mut self, _msg: NetworkMessage, ctx: &mut Self::Context) {
        // envia el who is leader
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
