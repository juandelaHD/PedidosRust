use actix::prelude::*;
use common::logger::Logger;
use common::network::communicator::Communicator;
use common::network::peer_types::PeerType;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use crate::messages::internal_messages::{RegisterConnection, RegisterConnectionManager};
use crate::server_actors::coordinator::Coordinator;
use tokio::io::AsyncBufReadExt;

pub struct Acceptor {
    addr: SocketAddr,
    coordinator_address: Addr<Coordinator>,
    logger: Arc<Logger>,
}

impl Acceptor {
    pub fn new(
        addr: SocketAddr,
        coordinator_address: Addr<Coordinator>,
    ) -> Self {
        Self {
            addr,
            coordinator_address,
            logger: Arc::new(Logger::new("ACCEPTOR")),
        }
    }
}

impl Actor for Acceptor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = self.addr;

        let acceptor_addr = ctx.address();
        let logger = self.logger.clone();

        ctx.spawn(
            async move {
                match TcpListener::bind(addr).await {
                    Ok(listener) => {
                        logger.info(format!("Acceptor started on {}", addr));

                        loop {
                            match listener.accept().await {
                                Ok((mut stream, remote_addr)) => {
                                    let mut peer_type_byte = [0u8; 1];

                                    match stream.read_exact(&mut peer_type_byte).await {
                                        Ok(_) => {
                                            if let Some(peer_type) =
                                                PeerType::from_u8(peer_type_byte[0])
                                            {
                                            let mut addr_line = String::new();
                                            let mut reader = tokio::io::BufReader::new(&mut stream);
                                            reader.read_line(&mut addr_line).await.unwrap_or_else(|_| {
                                                logger.info(format!(
                                                    "Error leyendo dirección desde {}",
                                                    remote_addr
                                                ));
                                                0
                                            });

                                            acceptor_addr.do_send(HandleConnection {
                                                stream,
                                                remote_addr,
                                                peer_type,
                                            });
                                            } else {
                                                logger.info(format!(
                                                    "Tipo de peer inválido desde {}",
                                                    remote_addr
                                                ));
                                            }
                                        }
                                        Err(e) => {
                                            logger.info(format!(
                                                "Error leyendo tipo de peer desde {}: {}",
                                                remote_addr, e
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    logger.info(format!("Error aceptando conexión: {}", e));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        logger.info(format!("Error al bindear TCP listener: {}", e));
                    }
                }
            }
            .into_actor(self),
        );
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct HandleConnection {
    stream: TcpStream,
    remote_addr: SocketAddr,
    peer_type: PeerType,
}

impl Handler<HandleConnection> for Acceptor {
    type Result = ();

    fn handle(&mut self, msg: HandleConnection, _: &mut Context<Self>) {
        let HandleConnection {
            stream,
            remote_addr,
            peer_type,
        } = msg;

        match peer_type {
            PeerType::CoordinatorType => {
                println!("Conexión de Coordinador desde {:?}", remote_addr);
                let communicator =
                    Communicator::new(stream, self.coordinator_address.clone(), peer_type);
                self.coordinator_address
                    .do_send(RegisterConnectionManager {
                        remote_addr, 
                        communicator,
                    });
            }
            PeerType::ClientType | PeerType::RestaurantType | PeerType::DeliveryType => {
                println!("Conexión de Usuario desde {:?}", remote_addr);
                let communicator =
                    Communicator::new(stream, self.coordinator_address.clone(), peer_type);
                self.coordinator_address.do_send(RegisterConnection {
                    client_addr:  remote_addr,
                    communicator,
                });
            }
            _ => {
                println!("Conexión de tipo desconocido desde {:?}", remote_addr);
            }
        }
    }
}
