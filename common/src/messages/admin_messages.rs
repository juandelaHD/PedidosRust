use actix::Message;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RegisterClient {
    pub id: String,
    pub addr: SocketAddr,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RegisterRestaurant {
    pub id: String,
    pub addr: SocketAddr,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RegisterDelivery {
    pub id: String,
    pub addr: SocketAddr,
}
