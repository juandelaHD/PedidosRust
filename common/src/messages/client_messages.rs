use crate::types::dtos::{ClientDTO, OrderDTO};
use actix::Message;
use serde::{Deserialize, Serialize};

/// Message sent by a client to request placing a new order.
///
/// ## Purpose
/// This message is used by the client to initiate an order request, sending the order details
/// to the coordinator or server for processing.
///
/// ## Contents
/// - `order`: The [`OrderDTO`] containing all relevant information about the order being placed.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RequestThisOrder {
    pub order: OrderDTO,
}

/// Message sent by a client to request a list of nearby restaurants.
///
/// ## Purpose
/// This message is used by the client to discover which restaurants are available in their vicinity,
/// typically to present options to the user before placing an order.
///
/// ## Contents
/// - `client`: The [`ClientDTO`] containing the client's information and location.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RequestNearbyRestaurants {
    pub client: ClientDTO,
}

/// Message sent to notify the client that their order has been finalized.
///
/// ## Purpose
/// This message is sent from the server or coordinator to the client to indicate that the order
/// process has been completed (e.g., delivered, cancelled, or otherwise finalized).
///
/// ## Contents
/// - `order`: The [`OrderDTO`] representing the finalized order.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct OrderFinalized {
    pub order: OrderDTO,
}

/// Message sent to inform the client of the expected delivery time for their order.
///
/// ## Purpose
/// This message is used to communicate an estimated delivery time to the client after an order
/// has been placed and accepted by a delivery agent.
///
/// ## Contents
/// - `order`: The [`OrderDTO`] for which the delivery time is being provided.
/// - `expected_time`: The expected delivery time in seconds.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct DeliveryExpectedTime {
    pub order: OrderDTO,
    pub expected_time: u64, // in seconds
}
