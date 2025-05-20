use actix::prelude::*;
use std::net::SocketAddr;
use std::sync::Arc;
use common::logger::{Logger};
use crate::server_actors::server_actor::Server;

pub struct Acceptor {
    pub addr: SocketAddr,
    pub logger: Arc<Logger>,
}

impl Acceptor {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr, logger: Arc::new(Logger::new("ACCEPTOR")) }
    }

    pub async fn start(&self) -> std::io::Result<()> {
        
        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        self.logger.info(format!(
            "Acceptor started, listening on {}",
            self.addr
        ));

        loop {
            match listener.accept().await {
                Ok((stream, client_addr)) => {
                    self.logger.info(format!(
                        "Acceptor received connection from {:?}",
                        client_addr
                    ));
                    Server::new(
                        stream,
                        self.addr,
                        client_addr,   
                    );
                }
                Err(e) => {
                    self.logger.error(format!(
                        "Acceptor failed accepting connection: {}",
                        e
                    ));
                }
            }
        }
    }
}
