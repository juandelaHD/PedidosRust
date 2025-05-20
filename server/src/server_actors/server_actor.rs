use actix::prelude::*;
use tokio::io::{split, BufReader, AsyncBufReadExt};
use std::net::SocketAddr;
use std::sync::Arc;
use common::logger::{Logger};
use common::network::socket_reader::{IncomingLine, SocketReader};
use common::network::socket_writer::{SocketWriter, TCPMessage};
use serde::Serialize;


pub struct Server {
    pub addr: SocketAddr,
    pub client_addr: SocketAddr,
    pub socket_writer: Option<Arc<Addr<SocketWriter>>>,    
    pub socket_reader: Option<Arc<Addr<SocketReader<Server>>>>,
    pub logger: Arc<Logger>,
}

impl Actor for Server {
    type Context = Context<Self>;
}

impl Server {
    pub fn new(
        stream: tokio::net::TcpStream,
        srv_addr: SocketAddr,
        client_addr: SocketAddr,
    ) -> Addr<Self> {
        Server::create(move |ctx| {            
            let (read_half, write_half) = split(stream);
            let socket_writer = Some(Arc::new(SocketWriter::new(write_half).start()));
            let socket_reader = Some(Arc::new(SocketReader::new(read_half, ctx.address()).start()));

            Server {
                addr: srv_addr,
                client_addr,
                socket_writer,
                socket_reader,
                logger: Arc::new(Logger::new(format!("SERVER {}", client_addr))),
            }
        })
    }
    
    fn send_message<T: Serialize>(&self, msg: &T) {
        if let Ok(json) = serde_json::to_string(msg) {
            if let Some(writer) = &self.socket_writer {
                writer.do_send(TCPMessage(json));
            } else {
                self.logger.warn("No socket_writer available");
            }
        } else {
            self.logger.warn("Failed to serialize message");
        }
    }
    
}

impl Handler<IncomingLine> for Server {
    type Result = ();
    fn handle(&mut self, msg: IncomingLine, _ctx: &mut Self::Context) -> Self::Result {
        print!("{}: Received message: {}\n", self.client_addr, msg.0);
        
    }
}


