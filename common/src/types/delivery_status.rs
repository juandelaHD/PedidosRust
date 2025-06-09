use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Available,           // Listo para recibir ofertas de pedidos
    WaitingConfirmation, // Esperando confirmación del restaurante (despues de aceptar un pedido)
    Delivering,          // En proceso de entrega
}
