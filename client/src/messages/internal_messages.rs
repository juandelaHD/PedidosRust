use actix::Message;
use common::types::restaurant_info::RestaurantInfo;

#[derive(Message)]
#[rtype(result = "()")]
pub struct SelectNearbyRestaurants {
    pub nearby_restaurants: Vec<RestaurantInfo>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SendThisOrder {
    pub selected_restaurant: String,
    pub selected_dish: String,
}

/// Mensajes que puede recibir el UIHandler
#[derive(Message)]
#[rtype(result = "()")]
pub enum UIMessage {
    ShowMessage(String),
    ShowOrderStatus(String),
}