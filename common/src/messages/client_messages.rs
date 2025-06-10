use actix::Message;
use serde::{Deserialize, Serialize};
use crate::types::{dtos::{ClientDTO, OrderDTO}, restaurant_info::RestaurantInfo};

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
pub struct NearbyRestaurants {
    pub restaurants: Vec<RestaurantInfo>,
}
