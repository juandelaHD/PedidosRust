use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use crate::types::delivery_status::DeliveryStatus;
use crate::types::order_status::OrderStatus;
use actix::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Message, Clone)]
#[serde(tag = "user_type")]
#[rtype(result = "()")]
pub enum UserDTO {
    Client(ClientDTO),
    Restaurant(RestaurantDTO),
    Delivery(DeliveryDTO),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientDTO {
    /// Posición actual del cliente en coordenadas 2D.
    pub client_position: (f32, f32),
    /// ID único del cliente.
    pub client_id: String,
    /// Pedido del cliente
    pub client_order: Option<OrderDTO>,
    /// Marca de tiempo que registra la última actualización del cliente.
    pub time_stamp: std::time::SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestaurantDTO {
    /// Posición actual del restaurante en coordenadas 2D.
    pub restaurant_position: (f32, f32),
    /// ID único del restaurante.
    pub restaurant_id: String,
    /// Pedidos autorizados por el PaymentGatewat pero no aceptados todavía
    /// por el restaurante
    pub authorized_orders: HashSet<OrderDTO>,
    /// Pedidos pendientes.
    pub pending_orders: HashSet<OrderDTO>,
    /// Marca de tiempo que registra la última actualización del restaurante.
    pub time_stamp: std::time::SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryDTO {
    /// Posición actual del delivery en coordenadas 2D.
    pub delivery_position: (f32, f32),
    /// ID único del delivery.
    pub delivery_id: String,
    /// ID del cliente actual asociado con el delivery (si existe).
    pub current_client_id: Option<String>,
    /// ID de la orden actual.
    pub current_order: Option<OrderDTO>,
    /// Estado actual del delivery.
    pub status: DeliveryStatus,
    /// Marca de tiempo que registra la última actualización del delivery.
    pub time_stamp: std::time::SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Posición del cliente que realizó la orden.
    pub client_position: (f32, f32), // Útil para calcular distancias
    /// Tiempo estimado de preparación de la orden en segundos.
    pub expected_delivery_time: u64,
    /// Marca de tiempo que registra la última actualización de la orden.
    pub time_stamp: std::time::SystemTime,
}

impl Eq for OrderDTO {}

impl PartialEq for OrderDTO {
    fn eq(&self, other: &Self) -> bool {
        self.order_id == other.order_id
    }
}

impl std::hash::Hash for OrderDTO {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.order_id.hash(state);
    }
}

#[derive(Debug, Serialize, Deserialize, Message, Clone)]
#[rtype(result = "()")]
pub struct Snapshot {
    /// Diccionario con información sobre clientes.
    pub clients: HashMap<String, ClientDTO>,
    /// Diccionario con información sobre restaurantes.
    pub restaurants: HashMap<String, RestaurantDTO>,
    /// Diccionario con información sobre deliverys.
    pub deliverys: HashMap<String, DeliveryDTO>,
    /// Diccionario de órdenes.
    pub orders: HashMap<u64, OrderDTO>,
    /// Deliveries que solicitaron aceptar órdenes.
    pub accepted_deliveries: HashMap<u64, HashSet<String>>,
    /// Índice del próximo log.
    pub next_log_id: u64,
    /// Índice de la mínima operación persistente en el log.
    pub min_persistent_log_index: u64,
}