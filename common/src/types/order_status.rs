use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderStatus {
    Requiested,
    Authorized,
    Pending,
    Preparing,
    ReadyForDelivery,
    Delivering,
    Delivered,
    Cancelled(String), // Optional reason
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderStatus::Requiested => write!(f, "Requiested"),
            OrderStatus::Authorized => write!(f, "Authorized"),
            OrderStatus::Pending => write!(f, "Pending"),
            OrderStatus::Preparing => write!(f, "Preparing"),
            OrderStatus::ReadyForDelivery => write!(f, "Ready for Delivery"),
            OrderStatus::Delivering => write!(f, "Delivering"),
            OrderStatus::Delivered => write!(f, "Delivered"),
            OrderStatus::Cancelled(reason) => write!(f, "Cancelled: {}", reason),
        }
    }
}
