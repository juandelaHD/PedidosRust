use crate::types::dtos::{ClientDTO, DeliveryDTO, OrderDTO};
use crate::types::restaurant_info::RestaurantInfo;
use actix::Message;
use serde::{Deserialize, Serialize};

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct NearbyRestaurants {
    pub client: ClientDTO,
    pub restaurants: Vec<RestaurantInfo>,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct NotifyOrderUpdated {
    pub peer_id: String,
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
    pub delivery_info: DeliveryDTO,
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
    pub delivery_info: DeliveryDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct NearbyDeliveries {
    pub order: OrderDTO,
    pub deliveries: Vec<DeliveryDTO>,
}
