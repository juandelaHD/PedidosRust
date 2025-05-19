use serde::{Deserialize, Serialize};

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