use crate::types::{
    dtos::{DeliveryDTO, OrderDTO},
    restaurant_info::RestaurantInfo,
};
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
pub struct RequestNearbyDelivery {
    pub order: OrderDTO,
    pub restaurant_info: RestaurantInfo,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct DeliveryAccepted {
    pub order: OrderDTO,
    pub delivery: DeliveryDTO,
}
