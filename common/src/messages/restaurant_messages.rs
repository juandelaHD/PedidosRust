use crate::types::{dtos::OrderDTO, restaurant_info::RestaurantInfo};
use actix::Message;
use serde::{Deserialize, Serialize};

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct UpdateOrderStatus {
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct CancelOrder {
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct OrderIsPreparing {
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RequestNearbyDelivery {
    pub order: OrderDTO,
    pub restaurant_info: RestaurantInfo,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct DeliverThisOrder {
    pub order: OrderDTO,
}
