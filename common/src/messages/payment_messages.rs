use crate::types::dtos::OrderDTO;
use actix::Message;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RequestAuthorization {
    pub origin_address: SocketAddr,
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AuthorizationResult {
    pub result: OrderDTO,
}
