use actix::Message;
use common::types::restaurant_info::RestaurantInfo;

/// Request message to fetch nearby restaurants.
///
/// This message updates the UI with available restaurant options for the user to select from.
///
/// Content:
/// - `nearby_restaurants`: A vector of `RestaurantInfo` containing details about each restaurant.
#[derive(Message)]
#[rtype(result = "()")]
pub struct SelectNearbyRestaurants {
    pub nearby_restaurants: Vec<RestaurantInfo>,
}

/// Request message to send an order to the selected restaurant.
///
/// This message is used to send the user's order details to the coordinator.
///
/// Content:
/// - `selected_restaurant`: The name of the restaurant where the order is placed.
/// - `selected_dish`: The name of the dish that the user has selected to order.
#[derive(Message)]
#[rtype(result = "()")]
pub struct SendThisOrder {
    pub selected_restaurant: String,
    pub selected_dish: String,
}

/// Enum: UIMessage
/// This enum defines the messages that the UIHandler can receive.
#[derive(Message)]
#[rtype(result = "()")]
pub enum UIMessage {
    ShowMessage(String),
    ShowOrderStatus(String),
}
