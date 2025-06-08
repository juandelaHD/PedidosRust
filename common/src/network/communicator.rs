use crate::messages::shared_messages::NetworkMessage;
use crate::network::peer_types::PeerType;
use crate::network::tcp_receiver::TCPReceiver;
use crate::network::tcp_sender::TCPSender;
use actix::prelude::*;
use std::sync::Arc;
use tokio::net::TcpStream;

pub struct Communicator<A>
where
    A: Actor<Context = Context<A>> + Handler<NetworkMessage>,
{
    pub stream: Option<TcpStream>,
    pub sender: Option<Arc<Addr<TCPSender>>>,
    pub receiver: Option<Arc<Addr<TCPReceiver<A>>>>,
    pub peer_type: PeerType, // Enum: Client, Restaurant, Delivery, Coordinator, Gateway
}

impl<A> Communicator<A>
where
    A: Actor<Context = Context<A>> + Handler<NetworkMessage>,
{
    pub fn new(tcp_stream: TcpStream, peer_type: PeerType) -> Self {
        Self {
            stream: Some(tcp_stream),
            sender: None,
            receiver: None,
            peer_type,
        }
    }
}
