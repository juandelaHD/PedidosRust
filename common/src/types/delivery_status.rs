use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Eq)]
pub enum DeliveryStatus {
    Reconnecting,        // Reconectando a un servidor
    Recovering,          // Recuperando estado de un pedido
    Available,           // Listo para recibir ofertas de pedidos
    WaitingConfirmation, // Esperando confirmación del restaurante (despues de aceptar un pedido)
    Delivering,          // En proceso de entrega
}
