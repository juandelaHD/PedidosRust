use std::{collections::HashSet, time::Instant};

use crate::types::delivery_status::DeliveryStatus;
use crate::types::order_status::OrderStatus;

pub struct ClientDTO {
    /// Posición actual del cliente en coordenadas 2D.
    pub client_position: (f32, f32),
    /// ID único del cliente.
    pub client_id: String,
    /// Pedido del cliente (id de alimento).
    pub client_order_id: Option<u64>,
    /// Marca de tiempo que registra la última actualización del cliente.
    pub time_stamp: Instant,
}

pub struct RestaurantDTO {
    /// Posición actual del restaurante en coordenadas 2D.
    pub restaurant_position: (f32, f32),
    /// ID único del restaurante.
    pub restaurant_id: String,
    /// Pedidos autorizados por el PaymentGatewat pero no aceptados todavía
    /// por el restaurante
    pub authorized_orders: HashSet<u64>,
    /// Pedidos pendientes.
    pub pending_orders: HashSet<u64>,
    /// Marca de tiempo que registra la última actualización del restaurante.
    pub time_stamp: Instant,
}

pub struct DeliveryDTO {
    /// Posición actual del delivery en coordenadas 2D.
    pub delivery_position: (f32, f32),
    /// ID único del delivery.
    pub delivery_id: String,
    /// ID del cliente actual asociado con el delivery (si existe).
    pub current_client_id: Option<String>,
    /// ID de la orden actual.
    pub current_order_id: Option<u64>,
    /// Estado actual del delivery.
    pub status: DeliveryStatus,
    /// Marca de tiempo que registra la última actualización del delivery.
    pub time_stamp: Instant,
}

pub struct OrderDTO {
    /// ID de la orden.
    pub order_id: u64,
    /// Nombre del plato seleccionado
    pub dish_name: String,
    /// ID del cliente asociado a la orden.
    pub client_id: String,
    /// ID del restaurante asociado a la orden.
    pub restaurant_id: String,
    /// ID del delivery asociado a la orden.
    pub delivery_id: Option<String>,
    /// Estado de la orden.
    pub status: OrderStatus,
    /// Marca de tiempo que registra la última actualización de la orden.
    pub time_stamp: Instant,
}
