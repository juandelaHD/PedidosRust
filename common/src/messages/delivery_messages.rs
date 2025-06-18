use crate::types::{
    dtos::{DeliveryDTO, OrderDTO},
    restaurant_info::RestaurantInfo,
};
use actix::Message;
use serde::{Deserialize, Serialize};

/// Message sent by a delivery agent to announce their availability.
///
/// # Purpose
/// Used by a delivery agent to inform the coordinator or system that they are available to take delivery assignments.
///
/// # Contents
/// - `delivery_info`: The [`DeliveryDTO`] containing information about the delivery agent.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct IAmAvailable {
    pub delivery_info: DeliveryDTO,
}

/// Message sent to notify that a delivery agent has accepted an order.
///
/// # Purpose
/// Used by a delivery agent to inform the system that they have accepted a specific order for delivery.
///
/// # Contents
/// - `order`: The [`OrderDTO`] representing the accepted order.
/// - `delivery_info`: The [`DeliveryDTO`] with details about the delivery agent.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AcceptedOrder {
    pub order: OrderDTO,
    pub delivery_info: DeliveryDTO,
}

/// Message sent to notify that an order has been delivered.
///
/// # Purpose
/// Used by a delivery agent to inform the system or coordinator that the order has been successfully delivered to the client.
///
/// # Contents
/// - `order`: The [`OrderDTO`] representing the delivered order.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct OrderDelivered {
    pub order: OrderDTO,
}

/// Message sent to a delivery agent instructing them to deliver a specific order.
///
/// # Purpose
/// Used by the coordinator or system to assign a delivery agent to deliver a particular order.
///
/// # Contents
/// - `order`: The [`OrderDTO`] to be delivered.
/// - `restaurant_info`: The [`RestaurantInfo`] with details about the restaurant where the order should be picked up.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct DeliverThisOrder {
    pub order: OrderDTO,
    pub restaurant_info: RestaurantInfo,
}

/// Message sent by a delivery agent to indicate they are currently delivering an order.
///
/// # Purpose
/// Used by a delivery agent to update the system about the ongoing delivery, including the expected delivery time.
///
/// # Contents
/// - `delivery_info`: The [`DeliveryDTO`] with details about the delivery agent.
/// - `expected_delivery_time`: The expected delivery time in seconds.
/// - `order`: The [`OrderDTO`] being delivered.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct IAmDelivering {
    pub delivery_info: DeliveryDTO,
    pub expected_delivery_time: u64,
    pub order: OrderDTO,
}
