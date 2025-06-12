use crate::types::dtos::OrderDTO;
use crate::types::restaurant_info::RestaurantInfo;
use actix::Message;
use serde::{Deserialize, Serialize};

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

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct NewOfferToDeliver {
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct DeliveryNoNeeded {
    pub order: OrderDTO,
}
