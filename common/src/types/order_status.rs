use serde::{Deserialize, Serialize};
use std::fmt;

/// Enum representing the status of an order
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderStatus {
    /// The order has been requested by a customer
    Requested,
    /// The order has been authorized by the PaymentGateway
    Authorized,
    /// The order has not been authorized by the PaymentGateway
    Unauthorized,
    /// The order has been accepted by the restaurant and is pending preparation by the kitchen
    Pending,
    /// The restaurant has accepted the order and is in the process of preparing it
    Preparing,
    /// The order is ready to be delivered to the customer
    ReadyForDelivery,
    /// The order is currently being delivered to the customer
    Delivering,
    /// The order has been delivered to the customer
    Delivered,
    /// The order has been cancelled
    Cancelled,
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderStatus::Requested => write!(f, "Requested"),
            OrderStatus::Unauthorized => write!(f, "Unauthorized"),
            OrderStatus::Authorized => write!(f, "Authorized"),
            OrderStatus::Pending => write!(f, "Pending"),
            OrderStatus::Preparing => write!(f, "Preparing"),
            OrderStatus::ReadyForDelivery => write!(f, "Ready for Delivery"),
            OrderStatus::Delivering => write!(f, "Delivering"),
            OrderStatus::Delivered => write!(f, "Delivered"),
            OrderStatus::Cancelled => write!(f, "Cancelled. Try again later."),
        }
    }
}
