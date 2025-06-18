use crate::types::dtos::{ClientDTO, DeliveryDTO, OrderDTO};
use crate::types::restaurant_info::RestaurantInfo;
use actix::Message;
use serde::{Deserialize, Serialize};

/// Message sent to a client with a list of nearby restaurants.
///
/// ## Purpose
/// Sent by the coordinator to inform a client about restaurants available in their vicinity.
///
/// ## Contents
/// - `client`: The [`ClientDTO`] representing the client who made the request.
/// - `restaurants`: A list of [`RestaurantInfo`] objects with details of each nearby restaurant.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct NearbyRestaurants {
    pub client: ClientDTO,
    pub restaurants: Vec<RestaurantInfo>,
}

/// Message sent to notify a peer (client, restaurant, or delivery) that an order has been updated.
///
/// ## Purpose
/// Used by the coordinator to inform a participant that the status or details of an order have changed.
///
/// ## Contents
/// - `peer_id`: The ID of the peer to notify.
/// - `order`: The updated [`OrderDTO`] for the order.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct NotifyOrderUpdated {
    pub peer_id: String,
    pub order: OrderDTO,
}

/// Message sent to a restaurant to notify about a new order.
///
/// ## Purpose
/// Used by the coordinator to inform a restaurant that a new order has been placed and needs processing.
///
/// ## Contents
/// - `order`: The [`OrderDTO`] representing the new order.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct NewOrder {
    pub order: OrderDTO,
}

/// Message sent to a restaurant to indicate a delivery agent is available for an order.
///
/// ## Purpose
/// Used by the coordinator to notify a restaurant that a delivery agent is available for a specific order.
///
/// ## Contents
/// - `order`: The [`OrderDTO`] for which delivery is available.
/// - `delivery_info`: The [`DeliveryDTO`] with details about the available delivery agent.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct DeliveryAvailable {
    pub order: OrderDTO,
    pub delivery_info: DeliveryDTO,
}

/// Message sent to a delivery agent to offer them a new order to deliver.
///
/// ## Purpose
/// Used by the coordinator to propose a delivery assignment to a delivery agent.
///
/// ## Contents
/// - `order`: The [`OrderDTO`] representing the order to be delivered.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct NewOfferToDeliver {
    pub order: OrderDTO,
}

/// Message sent to a delivery agent to indicate their services are not needed for an order.
///
/// ## Purpose
/// Used by the coordinator to inform a delivery agent that they are no longer needed for a specific order.
///
/// ## Contents
/// - `order`: The [`OrderDTO`] for which delivery is no longer needed.
/// - `delivery_info`: The [`DeliveryDTO`] with details about the delivery agent.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct DeliveryNoNeeded {
    pub order: OrderDTO,
    pub delivery_info: DeliveryDTO,
}

/// Message sent to a client with a list of available delivery agents for their order.
///
/// ## Purpose
/// Used by the coordinator to inform a client about which delivery agents are available for their order.
///
/// ## Contents
/// - `order`: The [`OrderDTO`] for which delivery agents are being proposed.
/// - `deliveries`: A list of [`DeliveryDTO`] objects representing available delivery agents.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct NearbyDeliveries {
    pub order: OrderDTO,
    pub deliveries: Vec<DeliveryDTO>,
}
