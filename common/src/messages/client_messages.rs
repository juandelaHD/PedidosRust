use actix::Message;
use serde::{Deserialize, Serialize};
use crate::types::dtos::{ClientDTO, OrderDTO};

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RequestThisOrder {
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RequestNearbyRestaurants {
    pub client: ClientDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct OrderFinalized {
    pub order: OrderDTO,
}
