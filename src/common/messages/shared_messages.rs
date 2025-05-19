use actix::Message;
use serde::{Deserialize, Serialize};
use crate::common::types::order_status::OrderStatus;

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct OrderStatusUpdate {
    pub order_id: String,
    pub status: OrderStatus,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct LocationUpdate {
    pub id: String,
    pub coords: (f32, f32),
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct PersistState;

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RecoverState;
