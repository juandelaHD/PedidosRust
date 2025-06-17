use actix::dev::ToEnvelope;
use actix::prelude::*;
// Update the path below if 'common' is a sibling module or crate; adjust as needed:
use crate::messages::shared_messages::NetworkMessage;
use tokio::io::{AsyncBufReadExt, BufReader, ReadHalf};
use tokio::net::TcpStream;

pub struct TCPReceiver<A: Actor + Handler<NetworkMessage>> {
    reader: Option<BufReader<ReadHalf<TcpStream>>>,
    destination: Addr<A>,
}

impl<A> TCPReceiver<A>
where
    A: Actor + Handler<NetworkMessage>,
{
    pub fn new(reader: ReadHalf<TcpStream>, destination: Addr<A>) -> Self {
        Self {
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

        ctx.spawn(
            async move {
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Ok(msg) = serde_json::from_str::<NetworkMessage>(&line) {
                        addr.do_send(msg);
                    } else {
                        panic!("This message cannot be deserialized: {}", line);
                    }
                }
            }
            .into_actor(self),
        );
    }
}
