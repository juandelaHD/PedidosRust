use actix::dev::ToEnvelope;
use actix::prelude::*;
// Update the path below if 'common' is a sibling module or crate; adjust as needed:
use crate::messages::shared_messages::Shutdown;
use crate::messages::shared_messages::{ConnectionClosed, NetworkMessage};
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, BufReader, ReadHalf};
use tokio::net::TcpStream;

/// The `TCPReceiver` actor reads incoming lines from a TCP stream,
/// deserializes them as [`NetworkMessage`]s, and forwards them to the destination actor.
///
/// ## Type Parameters
/// - `A`: The actor type that will receive the deserialized messages.
pub struct TCPReceiver<A: Actor + Handler<NetworkMessage>> {
    /// The remote peer's socket address.
    remote_addr: SocketAddr,
    /// The buffered reader for the TCP stream.
    reader: Option<BufReader<ReadHalf<TcpStream>>>,
    /// The Actix address of the destination actor.
    destination: Addr<A>,
}

impl<A> TCPReceiver<A>
where
    A: Actor + Handler<NetworkMessage>,
{
    /// Creates a new `TCPReceiver` for the given TCP stream read half.
    ///
    /// ## Arguments
    /// * `reader` - The read half of the TCP stream.
    /// * `remote_addr` - The address of the remote peer.
    /// * `destination` - The Actix address of the actor to forward messages to.
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
        let remote_addr = self.remote_addr;

        ctx.spawn(
            async move {
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    match serde_json::from_str::<NetworkMessage>(&line) {
                        Ok(msg) => {
                            if let Err(e) = addr.send(msg).await {
                                eprintln!("Failed to send NetworkMessage: {}", e);
                            }
                        }
                        Err(e) => {
                            panic!(
                                "This message cannot be deserialized: {}. Error: {}",
                                line, e
                            );
                        }
                    }
                }
                if let Err(e) = addr
                    .send(NetworkMessage::ConnectionClosed(ConnectionClosed {
                        remote_addr,
                    }))
                    .await
                {
                    eprintln!("Failed to send ConnectionClosed message: {}", e);
                }
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
