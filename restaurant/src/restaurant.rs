use actix::prelude::*;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;

use common::logger::Logger;
use common::network::communicator::Communicator;
use common::network::peer_types::PeerType;
use common::types::dtos::OrderDTO;



use crate::kitchen::Kitchen;

use common::messages::shared_messages::NetworkMessage;

pub struct Restaurant {
    /// Vector de direcciones de servidores
    pub servers: Vec<SocketAddr>,
    /// Identificador único del restaurante.
    pub restaurant_id: String,
    /// Posición actual del restaurante en coordenadas 2D.
    pub position: (f32, f32),
    /// Probabilidad de aceptar o rechazar un pedido.
    pub probability: f32,
    /// Canal de envío hacia la cocina.
    //pub kitchen_sender: Addr<Kitchen>,
    /// Comunicador asociado al Server.
    pub communicator: Option<Communicator<Restaurant>>,
    /// Guardamos el stream hasta que arranca el actor.
    pub pending_stream: Option<TcpStream>,
    /// Logger.
    pub logger: Logger,
}

impl Restaurant {
    pub async fn new(
        servers: Vec<SocketAddr>,
        id: String,
        position: (f32, f32),
        probability: f32,
        //kitchen_sender: Addr<Kitchen>,
    ) -> Self {
        let tcp_stream = connect(servers.clone()).await;
        let logger = Logger::new(format!("Restaurant {}", &id));
        if let Some(stream) = tcp_stream {
            Self {
                servers,
                restaurant_id: id,
                position,
                probability,
                //kitchen_sender,
                communicator: None,
                pending_stream: Some(stream),
                logger,
            }
        } else {
            panic!("Unable to connect to any server.");
        }
    }
}

impl Actor for Restaurant {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        if let Some(stream) = self.pending_stream.take() {
            self.communicator = Some(Communicator::new(
                stream,
                ctx.address(),
                PeerType::RestaurantType,
            ));
            self.logger.info("Communicator started for Restaurant");
        } else {
            self.logger.error("No stream available for Restaurant");
        }
    }
}

async fn connect(servers: Vec<SocketAddr>) -> Option<TcpStream> {
    for addr in servers {
        if let Ok(stream) = TcpStream::connect(addr).await {
            return Some(stream);
        }
    }
    None
}

impl Handler<NetworkMessage> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: NetworkMessage, _ctx: &mut Self::Context) {
        //self.logger.info(&format!("Received network message: {:?}", msg));
        // Acá después metés la lógica real.
    }
}