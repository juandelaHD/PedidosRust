use crate::messages::internal_messages::{RegisterConnection, RegisterConnectionWithCoordinator};
use crate::server_actors::coordinator::Coordinator;
use actix::prelude::*;
use colored::Color;
use common::logger::Logger;
use common::network::communicator::Communicator;
use common::network::peer_types::PeerType;
use std::net::SocketAddr;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;

/// The `Acceptor` actor listens for incoming TCP connections and registers them
/// with the appropriate coordinator actor based on the peer type.
///
/// ## Responsibilities
/// - Binds to a specified address and listens for incoming connections.
/// - Reads the peer type and remote address from each connection.
/// - Wraps each connection in a [`Communicator`] and registers it with the coordinator.
pub struct Acceptor {
    /// The address to bind and listen for incoming connections.
    addr: SocketAddr,
    /// The address of the coordinator actor to register connections with.
    coordinator_address: Addr<Coordinator>,
    /// Logger for acceptor events.
    logger: Logger,
}

impl Acceptor {
    /// Creates a new `Acceptor` instance.
    ///
    /// ## Arguments
    /// * `addr` - The socket address to bind to.
    /// * `coordinator_address` - The Actix address of the coordinator actor.
    pub fn new(addr: SocketAddr, coordinator_address: Addr<Coordinator>) -> Self {
        Self {
            addr,
            coordinator_address,
            logger: Logger::new("ACCEPTOR", Color::BrightBlack),
        }
    }
}

impl Actor for Acceptor {
    type Context = Context<Self>;

    /// Starts the acceptor, binding to the configured address and spawning a loop
    /// to accept and process incoming TCP connections.
    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = self.addr;

        let acceptor_addr = ctx.address();
        let logger = self.logger.clone();

        ctx.spawn(
            async move {
                match TcpListener::bind(addr).await {
                    Ok(listener) => {
                        logger.info(format!("Server Acceptor started on {}", addr));

                        loop {
                            match listener.accept().await {
                                Ok((mut stream, remote_addr)) => {
                                    let mut peer_type_byte = [0u8; 1];

                                    match stream.read_exact(&mut peer_type_byte).await {
                                        Ok(_) => {
                                            if let Some(peer_type) =
                                                PeerType::from_u8(peer_type_byte[0])
                                            {
                                                acceptor_addr.do_send(HandleConnection {
                                                    stream,
                                                    remote_addr,
                                                    peer_type,
                                                });
                                            } else {
                                                logger.info(format!(
                                                    "Received unsupported peer type byte from {}",
                                                    remote_addr
                                                ));
                                            }
                                        }
                                        Err(e) => {
                                            logger.info(format!(
                                                "Error while reading peer type from {}: {}",
                                                remote_addr, e
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    logger.info(format!("Error accepting connection: {}", e));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        logger.info(format!("Error binding listener: {}", e));
                    }
                }
            }
            .into_actor(self),
        );
    }
}

/// Internal message used to handle a new TCP connection.
///
/// ## Purpose
/// Contains the TCP stream, remote address, and peer type for the new connection.
#[derive(Message)]
#[rtype(result = "()")]
struct HandleConnection {
    stream: TcpStream,
    remote_addr: SocketAddr,
    peer_type: PeerType,
}

impl Handler<HandleConnection> for Acceptor {
    type Result = ();

    /// Handles a new incoming TCP connection by wrapping it in a [`Communicator`]
    /// and registering it with the coordinator actor.
    ///
    /// - For `CoordinatorType` peers, registers with `RegisterConnectionWithCoordinator`.
    /// - For `ClientType`, `RestaurantType`, and `DeliveryType` peers, registers with `RegisterConnection`.
    /// - Logs unsupported peer types.
    fn handle(&mut self, msg: HandleConnection, _: &mut Context<Self>) {
        let HandleConnection {
            stream,
            remote_addr,
            peer_type,
        } = msg;

        match peer_type {
            PeerType::CoordinatorType => {
                self.logger.info("Received connection from Coordinator. Registering...");
                let communicator =
                    Communicator::new(stream, self.coordinator_address.clone(), peer_type);
                self.coordinator_address
                    .do_send(RegisterConnectionWithCoordinator {
                        remote_addr,
                        communicator,
                    });
            }
            PeerType::ClientType | PeerType::RestaurantType | PeerType::DeliveryType => {
                self.logger.info("Received connection from Client/Restaurant/Delivery. Registering...");
                let communicator =
                    Communicator::new(stream, self.coordinator_address.clone(), peer_type);
                self.coordinator_address.do_send(RegisterConnection {
                    client_addr: remote_addr,
                    communicator,
                });
            }
            _ => {
                self.logger.info(format!(
                    "Received unsupported peer type {:?} from {}",
                    peer_type, remote_addr
                ));
            }
        }
    }
}
