use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderStatus {
    Requested,        // Se ha solicitado el pedido por el cliente
    Authorized,       // El pedido ha sido autorizado por el PaymentGateway
    Pending,          // El pedido está pendiente de ser aceptado por el restaurante
    Preparing,        // El restaurante ha aceptado el pedido y está en proceso de preparación
    ReadyForDelivery, // El pedido está listo para ser entregado
    Delivering,       // El pedido está siendo entregado
    Delivered,        // El pedido ha sido entregado al cliente
    Cancelled,        // El pedido ha sido cancelado
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderStatus::Requested => write!(f, "Requiested"),
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
