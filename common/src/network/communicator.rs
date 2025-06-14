use crate::messages::shared_messages::NetworkMessage;
use crate::network::peer_types::PeerType;
use crate::network::tcp_receiver::TCPReceiver;
use crate::network::tcp_sender::TCPSender;
use actix::prelude::*;
use std::sync::Arc;
use tokio::io::split;
use tokio::net::TcpStream;
use std::net::SocketAddr;

#[derive(Debug)]
pub struct Communicator<A>
where
    A: Actor<Context = Context<A>> + Handler<NetworkMessage>,
{
    pub local_address: SocketAddr,
    pub peer_address: SocketAddr,
    pub sender: Option<Arc<Addr<TCPSender>>>,
    pub receiver: Option<Arc<Addr<TCPReceiver<A>>>>,
    pub peer_type: PeerType, // Enum: Client, Restaurant, Delivery, Coordinator, Gateway
}

impl<A> Communicator<A>
where
    A: Actor<Context = Context<A>> + Handler<NetworkMessage>,
{
    pub fn new(tcp_stream: TcpStream, destination_address: Addr<A>, peer_type: PeerType) -> Self {
        let local_address = tcp_stream.local_addr().expect("Failed to get local address");
    let peer_address = tcp_stream.peer_addr().expect("Failed to get peer address");
        let (read_half, write_half) = split(tcp_stream);
        Self {
            local_address,
            peer_address,
            sender: Some(Arc::new(TCPSender::new(write_half).start())),
            receiver: Some(Arc::new(
                TCPReceiver::new(read_half, destination_address).start(),
            )),
            peer_type,
        }
    }
}
