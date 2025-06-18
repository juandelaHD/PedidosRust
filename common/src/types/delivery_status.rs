use serde::{Deserialize, Serialize};

/// Enum representing the delivery status of an order
#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Eq)]
pub enum DeliveryStatus {
    /// Reconnecting to a server
    Reconnecting,
    /// Recovering the state of an order
    Recovering,
    /// Available to accept orders
    Available,
    /// Waiting Restaurant confirmation after accepting an order
    WaitingConfirmation,
    /// Delivering an order
    Delivering,
}
