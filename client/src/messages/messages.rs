use actix::Message;
use common::types::order::OrderDTO;
use common::types::restaurant::RestaurantInfo;
use serde::{Deserialize, Serialize};

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RequestID;

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SendID {
    pub client_id: String,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SelectNearbyRestaurants {
    pub nearby_restaurants: Vec<RestaurantInfo>,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SendThisOrder {
    pub order: OrderDTO,
}
