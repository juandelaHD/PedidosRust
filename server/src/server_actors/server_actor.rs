use actix::prelude::*;
use common::messages::shared_messages::StartRunning;
use std::net::SocketAddr;
use std::sync::Arc;
use common::logger::Logger;
use common::network::socket_reader::{IncomingLine, SocketReader};
use common::network::socket_writer::{SocketWriter, TCPMessage};
use serde::Serialize;
use tokio::net::TcpStream;

pub struct Server {
    pub addr: SocketAddr,
    pub client_addr: SocketAddr,
    pub logger: Arc<Logger>,
    pub socket_writer: Option<Arc<Addr<SocketWriter>>>,
    pub stream: Option<TcpStream>,
}

impl Server {
    pub fn new_empty(
        stream: TcpStream,
        srv_addr: SocketAddr,
        client_addr: SocketAddr,
    ) -> Addr<Self> {
        Server::create(move |_ctx| {
            Server {
                addr: srv_addr,
                client_addr,
                logger: Arc::new(Logger::new(format!("SERVER {}", client_addr))),
                socket_writer: None,
                stream: Some(stream),
            }
        })
    }

    fn send_message<T: Serialize>(&self, msg: &T) {
        if let Some(writer) = &self.socket_writer {
            if let Ok(json) = serde_json::to_string(msg) {
                writer.do_send(TCPMessage(json));
            } else {
                self.logger.warn("Failed to serialize message");
            }
        } else {
            self.logger.warn("Attempted to send message without active writer");
        }
    }
}

impl Actor for Server {
    type Context = Context<Self>;
}

impl Handler<StartRunning> for Server {
    type Result = ();

    fn handle(&mut self, _msg: StartRunning, ctx: &mut Self::Context) {
        if let Some(stream) = self.stream.take() {
            let (read_half, write_half) = stream.into_split();

            let writer = Arc::new(SocketWriter::new(write_half).start());
            self.socket_writer = Some(writer.clone());

            let _reader = SocketReader::new(read_half, ctx.address()).start();

            self.logger.info("Server is now running".to_string());
        } else {
            self.logger.warn("StartRunning received, but no stream available");
        }
    }
}

impl Handler<IncomingLine> for Server {
    type Result = ();

    fn handle(&mut self, msg: IncomingLine, _ctx: &mut Self::Context) -> Self::Result {
        println!("{}: Received message: {}", self.client_addr, msg.0);
    }
}


