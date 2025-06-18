use actix::dev::ToEnvelope;
use actix::prelude::*;
// Update the path below if 'common' is a sibling module or crate; adjust as needed:
use crate::messages::shared_messages::Shutdown;
use crate::messages::shared_messages::{ConnectionClosed, NetworkMessage};
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, BufReader, ReadHalf};
use tokio::net::TcpStream;

pub struct TCPReceiver<A: Actor + Handler<NetworkMessage>> {
    remote_addr: SocketAddr,
    reader: Option<BufReader<ReadHalf<TcpStream>>>,
    destination: Addr<A>,
}

impl<A> TCPReceiver<A>
where
    A: Actor + Handler<NetworkMessage>,
{
    pub fn new(reader: ReadHalf<TcpStream>, remote_addr: SocketAddr, destination: Addr<A>) -> Self {
        Self {
            remote_addr,
            reader: Some(BufReader::new(reader)),
            destination,
        }
    }
}

impl<A> Actor for TCPReceiver<A>
where
    A: Actor + Handler<NetworkMessage> + 'static,
    A::Context: ToEnvelope<A, NetworkMessage>,
{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = self.destination.clone();
        let reader = self.reader.take().unwrap();
        let remote_addr = self.remote_addr.clone();

        ctx.spawn(
            async move {
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    match serde_json::from_str::<NetworkMessage>(&line) {
                        Ok(msg) => {
                            addr.do_send(msg);
                        }
                        Err(e) => {
                            panic!(
                                "This message cannot be deserialized: {}. Error: {}",
                                line, e
                            );
                        }
                    }
                }
                addr.do_send(NetworkMessage::ConnectionClosed(ConnectionClosed {
                    remote_addr,
                }));
            }
            .into_actor(self),
        );
    }
}

impl<A> Handler<Shutdown> for TCPReceiver<A>
where
    A: Actor + Handler<NetworkMessage> + 'static,
    A::Context: ToEnvelope<A, NetworkMessage>,
{
    type Result = ();

    fn handle(&mut self, _msg: Shutdown, ctx: &mut Self::Context) {
        println!("[TCPReceiver] Received Shutdown signal.");
        ctx.stop();
    }
}
