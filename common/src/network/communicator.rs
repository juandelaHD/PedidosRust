use crate::messages::shared_messages::NetworkMessage;
use crate::messages::shared_messages::Shutdown;
use crate::network::peer_types::PeerType;
use crate::network::tcp_receiver::TCPReceiver;
use crate::network::tcp_sender::TCPSender;
use actix::prelude::*;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::split;
use tokio::net::TcpStream;

/// The `Communicator` struct manages a TCP connection between two peers,
/// handling both sending and receiving of [`NetworkMessage`]s using Actix actors.
///
/// ## Type Parameters
/// - `A`: The actor type that will receive incoming [`NetworkMessage`]s.
#[derive(Debug)]
pub struct Communicator<A>
where
    A: Actor<Context = Context<A>> + Handler<NetworkMessage>,
{
    /// The local socket address of this peer.
    pub local_address: SocketAddr,
    /// The remote peer's socket address.
    pub peer_address: SocketAddr,
    /// The TCP sender actor for outgoing messages.
    pub sender: Option<Arc<Addr<TCPSender>>>,
    /// The TCP receiver actor for incoming messages.
    pub receiver: Option<Arc<Addr<TCPReceiver<A>>>>,
    /// The type of the remote peer.
    pub peer_type: PeerType, // Enum: Client, Restaurant, Delivery, Coordinator, Gateway
}

impl<A> Communicator<A>
where
    A: Actor<Context = Context<A>> + Handler<NetworkMessage>,
{
    /// Creates a new `Communicator` for a TCP connection.
    ///
    /// ## Arguments
    /// * `tcp_stream` - The established TCP stream.
    /// * `destination_address` - The Actix address of the actor that will receive messages.
    /// * `peer_type` - The type of the remote peer.
    pub fn new(tcp_stream: TcpStream, destination_address: Addr<A>, peer_type: PeerType) -> Self {
        let local_address = tcp_stream
            .local_addr()
            .expect("Failed to get local address");
        let peer_address = tcp_stream.peer_addr().expect("Failed to get peer address");
        let (read_half, write_half) = split(tcp_stream);
        Self {
            local_address,
            peer_address,
            sender: Some(Arc::new(TCPSender::new(write_half).start())),
            receiver: Some(Arc::new(
                TCPReceiver::new(read_half, peer_address, destination_address).start(),
            )),
            peer_type,
        }
    }
}

impl<A> Communicator<A>
where
    A: Actor<Context = Context<A>> + Handler<NetworkMessage>,
{
    /// Shuts down the sender and receiver actors, closing the connection.
    pub fn shutdown(&mut self) {
        if let Some(sender) = self.sender.take() {
            sender.do_send(Shutdown);
        }

        if let Some(receiver) = self.receiver.take() {
            receiver.do_send(Shutdown);
        }

        println!(
            "[Communicator] Shutdown initiated for peer {}",
            self.peer_address
        );
    }
}

impl<A> Drop for Communicator<A>
where
    A: Actor<Context = Context<A>> + Handler<NetworkMessage>,
{
    fn drop(&mut self) {
        self.shutdown();
    }
}
