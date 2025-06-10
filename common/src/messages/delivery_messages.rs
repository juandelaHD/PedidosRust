use actix::Message;
use serde::{Deserialize, Serialize};

use crate::types::dtos::{OrderDTO, DeliveryDTO};
use crate::types::restaurant_info::RestaurantInfo;

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
    pub restaurant_info: RestaurantInfo,
}

