use crate::payment::PaymentGateway;
use actix::prelude::*;
use common::logger::Logger;
use common::network::communicator::Communicator;
use common::network::peer_types::PeerType;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;

pub struct PaymentAcceptor {
    addr: SocketAddr,
    payment_gateway_addr: Addr<PaymentGateway>,
    logger: Arc<Logger>,
}

impl PaymentAcceptor {
    pub fn new(addr: SocketAddr, payment_gateway_addr: Addr<PaymentGateway>) -> Self {
        Self {
            addr,
            payment_gateway_addr,
            logger: Arc::new(Logger::new("Payment ACCEPTOR")),
        }
    }
}

impl Actor for PaymentAcceptor {
    type Context = Context<Self>;

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
                                                let mut addr_line = String::new();
                                                let mut reader =
                                                    tokio::io::BufReader::new(&mut stream);
                                                reader
                                                    .read_line(&mut addr_line)
                                                    .await
                                                    .unwrap_or_else(|_| {
                                                        logger.info(format!(
                                                            "Error reading address line from {}",
                                                            remote_addr
                                                        ));
                                                        0
                                                    });

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

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct RegisterConnection {
    pub client_addr: SocketAddr,
    pub communicator: Communicator<PaymentGateway>,
}

#[derive(Message)]
#[rtype(result = "()")]
struct HandleConnection {
    stream: TcpStream,
    remote_addr: SocketAddr,
    peer_type: PeerType,
}

impl Handler<HandleConnection> for PaymentAcceptor {
    type Result = ();

    fn handle(&mut self, msg: HandleConnection, _: &mut Context<Self>) {
        let HandleConnection {
            stream,
            remote_addr,
            peer_type,
        } = msg;

        self.logger.info(format!(
            "New connection from {} with peer type {:?}",
            remote_addr, peer_type
        ));
        let communicator = Communicator::new(stream, self.payment_gateway_addr.clone(), peer_type);
        self.payment_gateway_addr.do_send(RegisterConnection {
            client_addr: remote_addr,
            communicator,
        });
    }
}
