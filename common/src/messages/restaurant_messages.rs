use crate::types::{
    dtos::{DeliveryDTO, OrderDTO},
    restaurant_info::RestaurantInfo,
};
use actix::Message;
use serde::{Deserialize, Serialize};

/// Message sent to update the status of an order at a restaurant.
///
/// ## Purpose
/// Used by the system to inform a restaurant of a status change for a specific order.
///
/// ## Contents
/// - `order`: The [`OrderDTO`] representing the order whose status has changed.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct UpdateOrderStatus {
    pub order: OrderDTO,
}

/// Message sent to cancel an order at a restaurant.
///
/// ## Purpose
/// Used by the system to instruct a restaurant to cancel a specific order.
///
/// ## Contents
/// - `order`: The [`OrderDTO`] representing the order to be cancelled.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct CancelOrder {
    pub order: OrderDTO,
}

/// Message sent by a restaurant to request a list of nearby delivery agents.
///
/// ## Purpose
/// Used by a restaurant to discover available delivery agents for a specific order.
///
/// ## Contents
/// - `order`: The [`OrderDTO`] for which delivery is needed.
/// - `restaurant_info`: The [`RestaurantInfo`] with details about the restaurant.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RequestNearbyDelivery {
    pub order: OrderDTO,
    pub restaurant_info: RestaurantInfo,
}

/// Message sent to notify that a delivery agent has accepted a delivery for an order.
///
/// ## Purpose
/// Used by the system to inform a restaurant that a delivery agent has accepted a delivery assignment.
///
/// ## Contents
/// - `order`: The [`OrderDTO`] for which delivery was accepted.
/// - `delivery`: The [`DeliveryDTO`] with details about the delivery agent.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct DeliveryAccepted {
    pub order: OrderDTO,
    pub delivery: DeliveryDTO,
}
