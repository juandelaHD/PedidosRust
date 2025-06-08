use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderStatus {
    Created,
    Confirmed,
    Preparing,
    Ready,
    PickedUp,
    Delivering,
    Delivered,
    Cancelled(String), // Optional reason
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderStatus::Created => write!(f, "Created"),
            OrderStatus::Confirmed => write!(f, "Confirmed"),
            OrderStatus::Preparing => write!(f, "Preparing"),
            OrderStatus::Ready => write!(f, "Ready"),
            OrderStatus::PickedUp => write!(f, "Picked Up"),
            OrderStatus::Delivering => write!(f, "Delivering"),
            OrderStatus::Delivered => write!(f, "Delivered"),
            OrderStatus::Cancelled(reason) => write!(f, "Cancelled ({})", reason),
        }
    }
}
