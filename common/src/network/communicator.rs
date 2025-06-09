use crate::messages::shared_messages::NetworkMessage;
use crate::network::peer_types::PeerType;
use crate::network::tcp_receiver::TCPReceiver;
use crate::network::tcp_sender::TCPSender;
use actix::prelude::*;
use std::sync::Arc;
use tokio::io::split;
use tokio::net::TcpStream;

#[derive(Debug)]
pub struct Communicator<A>
where
    A: Actor<Context = Context<A>> + Handler<NetworkMessage>,
{
    pub sender: Option<Arc<Addr<TCPSender>>>,
    pub receiver: Option<Arc<Addr<TCPReceiver<A>>>>,
    pub peer_type: PeerType, // Enum: Client, Restaurant, Delivery, Coordinator, Gateway
}

impl<A> Communicator<A>
where
    A: Actor<Context = Context<A>> + Handler<NetworkMessage>,
{
    pub fn new(tcp_stream: TcpStream, destination_address: Addr<A>, peer_type: PeerType) -> Self {
        let (read_half, write_half) = split(tcp_stream);
        Self {
            sender: Some(Arc::new(TCPSender::new(write_half).start())),
            receiver: Some(Arc::new(
                TCPReceiver::new(read_half, destination_address).start(),
            )),
            peer_type,
        }
    }
}
