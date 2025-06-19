use crate::payment::PaymentGateway;
use actix::prelude::*;
use colored::Color;
use common::logger::Logger;
use common::network::communicator::Communicator;
use common::network::peer_types::PeerType;
use std::net::SocketAddr;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;

/// The `PaymentAcceptor` actor listens for incoming TCP connections and registers them
/// with the [`PaymentGateway`] actor.
///
/// # Responsibilities
/// - Binds to a specified address and listens for incoming connections.
/// - Determines the peer type and remote address for each connection.
/// - Wraps each connection in a [`Communicator`] and registers it with the payment gateway.
pub struct PaymentAcceptor {
    /// The address to bind and listen for incoming connections.
    addr: SocketAddr,
    /// Address of the [`PaymentGateway`] actor to register new connections.
    payment_gateway_addr: Addr<PaymentGateway>,
    /// Logger for payment acceptor events.
    logger: Logger,
}

impl PaymentAcceptor {
    /// Creates a new `PaymentAcceptor`.
    ///
    /// # Arguments
    /// * `addr` - The socket address to bind to.
    /// * `payment_gateway_addr` - The address of the [`PaymentGateway`] actor.
    pub fn new(addr: SocketAddr, payment_gateway_addr: Addr<PaymentGateway>) -> Self {
        Self {
            addr,
            payment_gateway_addr,
            logger: Logger::new("Payment ACCEPTOR", Color::BrightBlack),
        }
    }
}

impl Actor for PaymentAcceptor {
    type Context = Context<Self>;

    /// Called when the actor starts.
    ///
    /// Binds to the configured address and enters an asynchronous loop to accept incoming TCP connections.
    /// For each connection, determines the peer type and remote address, then registers the connection
    /// with the payment gateway.
    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = self.addr;

        let payment_acceptor_addr = ctx.address();
        let logger = self.logger.clone();

        ctx.spawn(
            async move {
                match TcpListener::bind(addr).await {
                    Ok(listener) => {
                        logger.info(format!("Payment acceptor started on {}", addr));

                        loop {
                            match listener.accept().await {
                                Ok((mut stream, remote_addr)) => {
                                    let mut peer_type_byte = [0u8; 1];

                                    match stream.read_exact(&mut peer_type_byte).await {
                                        Ok(_) => {
                                            if let Some(peer_type) =
                                                PeerType::from_u8(peer_type_byte[0])
                                            {
                                                payment_acceptor_addr.do_send(HandleConnection {
                                                    stream,
                                                    remote_addr,
                                                    peer_type,
                                                });
                                            } else {
                                                logger.info(format!(
                                                    "Unknown peer type byte from {}",
                                                    remote_addr
                                                ));
                                            }
                                        }
                                        Err(e) => {
                                            logger.info(format!(
                                                "Error reading peer type from {}: {}",
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
                        logger.info(format!("Error binding to {}: {}", addr, e));
                    }
                }
            }
            .into_actor(self),
        );
    }
}

/// Message sent to the [`PaymentGateway`] to register a new connection.
///
/// Contains the remote address and the [`Communicator`] for the connection.
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct RegisterConnection {
    /// The remote address of the connected peer.
    pub client_addr: SocketAddr,
    /// The communicator for the connection.
    pub communicator: Communicator<PaymentGateway>,
}

/// Internal message used to handle a new TCP connection.
///
/// Contains the TCP stream, remote address, and peer type.
#[derive(Message)]
#[rtype(result = "()")]
struct HandleConnection {
    /// The TCP stream for the connection.
    stream: TcpStream,
    /// The remote address of the peer.
    remote_addr: SocketAddr,
    /// The type of peer that connected.
    peer_type: PeerType,
}

/// Handles [`HandleConnection`] messages.
///
/// Wraps the TCP stream in a [`Communicator`] and registers it with the [`PaymentGateway`] actor.
impl Handler<HandleConnection> for PaymentAcceptor {
    type Result = ();

    fn handle(&mut self, msg: HandleConnection, _: &mut Context<Self>) {
        let HandleConnection {
            stream,
            remote_addr,
            peer_type,
        } = msg;

        self.logger
            .info(format!("New connection from {}", remote_addr));
        let communicator = Communicator::new(stream, self.payment_gateway_addr.clone(), peer_type);
        self.payment_gateway_addr.do_send(RegisterConnection {
            client_addr: remote_addr,
            communicator,
        });
    }
}

impl Drop for PaymentAcceptor {
    /// Called when the actor is dropped.
    ///
    /// Logs that the payment acceptor is shutting down.
    fn drop(&mut self) {
        self.logger.info("Payment acceptor shutting down");
    }
}
