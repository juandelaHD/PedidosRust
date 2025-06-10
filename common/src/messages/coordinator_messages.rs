use actix::Message;
use serde::{Deserialize, Serialize};
use crate::types::dtos::OrderDTO;
use crate::types::restaurant_info::RestaurantInfo;

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct NearbyRestaurants {
    pub restaurants: Vec<RestaurantInfo>,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AuthorizationResult {
    pub result: Result<(), String>,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct NotifyOrderUpdated {
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct NewOrder {
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct DeliveryAvailable {
    pub order: OrderDTO,
}