use crate::types::dtos::{DeliveryDTO, OrderDTO};
use actix::Message;
use serde::{Deserialize, Serialize};

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct IAmAvailable {
    pub delivery_info: DeliveryDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AcceptOrder {
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct OrderDelivered {
    pub order: OrderDTO,
}
