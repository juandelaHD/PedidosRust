// use actix::prelude::*;
// use std::net::SocketAddr;
// use std::sync::Arc;
// use tokio::net::TcpStream;

// use common::logger::Logger;
// use common::messages::shared_messages::NetworkMessage;
// use common::messages::shared_messages::StartRunning;
// use common::network::communicator::Communicator;
// use common::network::peer_types::PeerType;
// use common::network::tcp_receiver::TCPReceiver;
// use common::network::tcp_sender::TCPSender;
// use common::types::delivery_status::DeliveryStatus;
// use common::types::dtos::OrderDTO;

// pub struct Delivery {
//     /// Vector de direcciones de servidores
//     pub servers: Vec<SocketAddr>,
//     /// Identificador único del delivery.
//     pub delivery_id: String,
//     /// Posición actual del delivery.
//     pub position: (f32, f32),
//     /// Estado actual del delivery: Disponible, Ocupado, Entregando.
//     pub status: DeliveryStatus,
//     //  Probabilidad de que rechace un pedido disponible de un restaurante.
//     pub probability: f32,
//     /// Pedido actual en curso, si lo hay.
//     pub current_order: Option<OrderDTO>,
//     /// Comunicador asociado al Server.
//     pub communicator: Communicator<Delivery>,

//     pub logger: Logger,
// }

// impl Delivery {
//     pub async fn new(
//         servers: Vec<SocketAddr>,
//         id: String,
//         position: (f32, f32),
//         probability: f32,
//     ) -> Self {
//         let tcp_stream: Option<TcpStream> = connect(servers.clone()).await;
//         let logger = Logger::new(format!("Delivery {}", &id));
//         if let Some(stream) = tcp_stream {
//             Self {
//                 servers,
//                 delivery_id: id,
//                 position,
//                 status: DeliveryStatus::Available,
//                 probability,
//                 current_order: None,
//                 communicator: Communicator::new(stream, PeerType::DeliveryType), // aca falta agregarle el context del delivery
//                 logger,
//             }
//         } else {
//             panic!("Unable to connect to any server.");
//         }
//     }
// }

// impl Actor for Delivery {
//     type Context = Context<Self>;

//     fn started(&mut self, ctx: &mut Self::Context) {
//         if let Some(stream) = self.communicator.stream.take() {
//             let (read_half, write_half) = tokio::io::split(stream);

//             self.communicator.sender = Some(Arc::new(TCPSender::new(write_half).start()));
//             self.communicator.receiver =
//                 Some(Arc::new(TCPReceiver::new(read_half, ctx.address()).start()));
//         } else {
//             self.logger.error("No stream available");
//         }
//     }
// }

// impl Handler<StartRunning> for Delivery {
//     type Result = ();

//     fn handle(&mut self, _msg: StartRunning, ctx: &mut Self::Context) {
//         // envia el who is leader
//     }
// }

// impl Handler<NetworkMessage> for Delivery {
//     type Result = ();

//     fn handle(&mut self, _msg: NetworkMessage, ctx: &mut Self::Context) {
//         // envia el who is leader
//     }
// }

// async fn connect(servers: Vec<SocketAddr>) -> Option<TcpStream> {
//     // Aca deberia preguntarle sobre informacion si se reconectó?
//     for addr in servers {
//         if let Ok(stream) = TcpStream::connect(addr).await {
//             return Some(stream);
//         }
//     }
//     None
// }
