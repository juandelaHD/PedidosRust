use crate::types::{
    dtos::{DeliveryDTO, OrderDTO},
    restaurant_info::RestaurantInfo,
};
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

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct DeliverThisOrder {
    pub order: OrderDTO,
    pub restaurant_info: RestaurantInfo,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct IAmDelivering {
    pub delivery_info: DeliveryDTO,
    pub expected_delivery_time: u64,
    pub order: OrderDTO,
}
