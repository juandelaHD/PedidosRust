use std::collections::HashSet;

use crate::types::order_status::OrderStatus;
use crate::{bimap::BiMap, types::delivery_status::DeliveryStatus};
use actix::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::HashMap;

/// Custom serialization for BiMap<u64, String>
mod bimap_u64_string_serde {
    use super::BiMap;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S>(bimap: &BiMap<u64, String>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convert u64 keys to strings for JSON serialization
        let string_map: HashMap<String, String> = bimap
            .keys()
            .zip(bimap.values())
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        string_map.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BiMap<u64, String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string_map: HashMap<String, String> = HashMap::deserialize(deserializer)?;
        let mut bimap = BiMap::new();
        for (k_str, v) in string_map {
            match k_str.parse::<u64>() {
                Ok(k) => {
                    bimap.insert(k, v);
                }
                Err(_) => {
                    return Err(serde::de::Error::custom(format!("Invalid u64 key: {}", k_str)));
                }
            }
        }
        Ok(bimap)
    }
}

/// Data Tranfer Object to represent different types of users in the system.
#[derive(Debug, Serialize, Deserialize, Message, Clone)]
#[serde(tag = "user_type")]
#[rtype(result = "()")]
pub enum UserDTO {
    /// Represents a Client (user who rquests orders).
    Client(ClientDTO),
    /// Represents a Restaurant (user who prepares orders).
    Restaurant(RestaurantDTO),
    /// Represents a Delivery (user who brings orders to clients).
    Delivery(DeliveryDTO),
}

/// Data Transfer Object to represent a client in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientDTO {
    /// Position of the client in 2D coordinates.
    pub client_position: (f32, f32),
    /// Unique ID of the client.
    pub client_id: String,
    /// User Order associated with the client (if any).
    pub client_order: Option<OrderDTO>,
    /// Timestamp that records the last update of the client.
    pub time_stamp: std::time::SystemTime,
}

/// Data Transfer Object to represent a restaurant in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestaurantDTO {
    /// Position of the restaurant in 2D coordinates.
    pub restaurant_position: (f32, f32),
    /// Unique ID of the restaurant.
    pub restaurant_id: String,
    /// Orders that the payment system has authorized for the restaurant, not yet accepted by the restaurant.
    pub authorized_orders: HashSet<OrderDTO>,
    /// Pending orders that the restaurant has not yet prepared.
    pub pending_orders: HashSet<OrderDTO>,
    /// Timestamp that records the last update of the restaurant.
    pub time_stamp: std::time::SystemTime,
}

/// Data Transfer Object to represent a delivery user in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryDTO {
    /// Position of the restaurant in 2D coordinates.
    pub delivery_position: (f32, f32),
    /// Unique ID of the delivery user.
    pub delivery_id: String,
    /// Unique ID of the client currently being served by the delivery. (None if not available).
    pub current_client_id: Option<String>,
    /// Unique ID of the order being delivered (None if available).
    pub current_order: Option<OrderDTO>,
    /// State of delivery user
    pub status: DeliveryStatus,
    /// Timestamp that records the last update of the delivery user.
    pub time_stamp: std::time::SystemTime,
}

/// Data Transfer Object to represent an order in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderDTO {
    /// Unique ID of the order.
    pub order_id: u64,
    /// Name of the dish associated with the order.
    pub dish_name: String,
    /// Unique ID of the client who placed the order.
    pub client_id: String,
    /// Unique ID of the restaurant that will prepare the order.
    pub restaurant_id: String,
    /// Unique ID of the delivery user assigned to deliver the order (None if not being delivered).
    pub delivery_id: Option<String>,
    /// State of the order.
    pub status: OrderStatus,
    /// Postion of the client in 2D coordinates.
    pub client_position: (f32, f32),
    /// Estimated time for the order to be delivered.
    pub expected_delivery_time: u64,
    /// Timestamp that records the last update of the order.
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

/// Data Transfer Object to represent a snapshot of the system state.
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Message, Clone)]
#[rtype(result = "()")]
pub struct Snapshot {
    /// Dictionary with information about clients.
    pub clients: HashMap<String, ClientDTO>,
    /// Dictionary with information about restaurants.
    pub restaurants: HashMap<String, RestaurantDTO>,
    /// Dictionary with information about deliveries.
    pub deliverys: HashMap<String, DeliveryDTO>,
    /// Dictionary with information about orders.
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub orders: HashMap<u64, OrderDTO>,
    /// BiMap of accepted deliveries
    #[serde(with = "bimap_u64_string_serde")]
    pub accepted_deliveries: BiMap<u64, String>,
    /// Index of the next log
    pub next_log_id: u64,
    /// Index of the minimum persistent log
    pub min_persistent_log_index: u64,
}
