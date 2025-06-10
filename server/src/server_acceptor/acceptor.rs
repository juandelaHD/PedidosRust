use actix::prelude::*;
use common::logger::Logger;
use common::network::communicator::Communicator;
use common::network::peer_types::PeerType;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::messages::m::RegisterConnection;
use crate::server_actors::server_actor::Coordinator;

pub struct Acceptor {
    addr: SocketAddr,
    coordinator_address: Addr<Coordinator>,
    logger: Arc<Logger>,
}

impl Acceptor {
    pub fn new(addr: SocketAddr, coordinator_address: Addr<Coordinator>) -> Self {
        Self {
            addr,
            coordinator_address,
            logger: Arc::new(Logger::new("ACCEPTOR")),
        }
    }
}

// Hacemos que Acceptor sea un Actor
impl Actor for Acceptor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = self.addr;
        let coordinator_address = self.coordinator_address.clone();
        let logger = self.logger.clone();

        // Spawn del future async dentro del contexto de Actix
        ctx.spawn(
            async move {
                match TcpListener::bind(addr).await {
                    Ok(listener) => {
                        logger.info(format!("Acceptor started, listening on {}", addr));

                        loop {
                            match listener.accept().await {
                                Ok((stream, client_addr)) => {
                                    logger
                                        .info(format!("Accepted connection from {}", client_addr));
                                    let communicator = Communicator::new(
                                        stream,
                                        coordinator_address.clone(),
                                        PeerType::ClientType,
                                    );
                                    coordinator_address.do_send(RegisterConnection {
                                        client_addr,
                                        communicator,
                                    });
                                }
                                Err(e) => {
                                    logger.warn(format!("Failed to accept connection: {}", e));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        logger.error(format!("Failed to bind to {}: {}", addr, e));
                    }
                }
            }
            .into_actor(self),
        );
    }
}
