use crate::types::delivery_status::DeliveryStatus;
use crate::types::dtos::ClientDTO;
use crate::types::dtos::DeliveryDTO;
use crate::types::dtos::OrderDTO;
use crate::types::dtos::RestaurantDTO;
use crate::types::order_status::OrderStatus;
use crate::types::restaurant_info::RestaurantInfo;
use actix::Message;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/////////////////////////////////////////////////////////////////////
// Mensajes del storage
/////////////////////////////////////////////////////////////////////

/// Message to apply a batch of storage updates.
///
/// ## Purpose
/// Used to apply a list of storage log updates, typically after receiving them from the coordinator manager.
///
/// ## Contents
/// - `is_leader`: Indicates if the node is the leader.
/// - `updates`: A list of (log index, [`StorageLogMessage`]) tuples to apply.
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct ApplyStorageUpdates {
    pub is_leader: bool,
    pub updates: Vec<(u64, StorageLogMessage)>,
}

/// Message to set the storage updates log.
///
/// ## Purpose
/// Used to replace the current storage log with a new one.
///
/// ## Contents
/// - `updates_log`: The new storage log as a map from log index to [`StorageLogMessage`].
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetStorageUpdatesLog {
    pub updates_log: HashMap<u64, StorageLogMessage>,
}

/// Enum representing all possible storage log messages.
///
/// ## Purpose
/// Encapsulates all operations that can be recorded in the storage log for recovery and synchronization.
///
/// ## Contents
/// Each variant wraps a specific message struct related to storage operations.
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[serde(tag = "storage_message")]
#[rtype(result = "()")]
pub enum StorageLogMessage {
    AddClient(AddClient),
    AddRestaurant(AddRestaurant),
    AddDelivery(AddDelivery),
    RemoveClient(RemoveClient),
    RemoveRestaurant(RemoveRestaurant),
    RemoveDelivery(RemoveDelivery),
    SetDeliveryPosition(SetDeliveryPosition),
    SetCurrentClientToDelivery(SetCurrentClientToDelivery),
    SetDeliveryStatus(SetDeliveryStatus),

    /// mensajes con order service
    AddOrder(AddOrder),
    RemoveOrder(RemoveOrder),
    AddAuthorizedOrderToRestaurant(AddAuthorizedOrderToRestaurant),
    AddPendingOrderToRestaurant(AddPendingOrderToRestaurant),
    RemoveAuthorizedOrderToRestaurant(RemoveAuthorizedOrderToRestaurant),
    RemovePendingOrderToRestaurant(RemovePendingOrderToRestaurant),
    SetCurrentOrderToDelivery(SetCurrentOrderToDelivery),
    SetDeliveryToOrder(SetDeliveryToOrder),
    SetOrderStatus(SetOrderStatus),
    SetOrderExpectedTime(SetOrderExpectedTime),

    InsertAcceptedDelivery(InsertAcceptedDelivery),
    RemoveAcceptedDeliveries(RemoveAcceptedDeliveries),
}

/// Message to add a new client to storage.
///
/// ## Purpose
/// Used to record the addition of a new client.
///
/// ## Contents
/// - `client`: The [`ClientDTO`] representing the client.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AddClient {
    pub client: ClientDTO,
}

/// Message to add a new restaurant to storage.
///
/// ## Purpose
/// Used to record the addition of a new restaurant.
///
/// ## Contents
/// - `restaurant`: The [`RestaurantDTO`] representing the restaurant.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AddRestaurant {
    pub restaurant: RestaurantDTO,
}

/// Message to add a new delivery agent to storage.
///
/// ## Purpose
/// Used to record the addition of a new delivery agent.
///
/// ## Contents
/// - `delivery`: The [`DeliveryDTO`] representing the delivery agent.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AddDelivery {
    pub delivery: DeliveryDTO,
}

/// Message to add a new order to storage.
///
/// ## Purpose
/// Used to record the addition of a new order.
///
/// ## Contents
/// - `order`: The [`OrderDTO`] representing the order.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AddOrder {
    pub order: OrderDTO,
}

/// Message to get a client by ID from storage.
///
/// ## Purpose
/// Used to retrieve a client by their ID.
///
/// ## Contents
/// - `client_id`: The ID of the client to retrieve.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Option<ClientDTO>")]
pub struct GetClient {
    pub client_id: String,
}

/// Message to get a restaurant by ID from storage.
///
/// ## Purpose
/// Used to retrieve a restaurant by its ID.
///
/// ## Contents
/// - `restaurant_id`: The ID of the restaurant to retrieve.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Option<RestaurantDTO>")]
pub struct GetRestaurant {
    pub restaurant_id: String,
}

/// Message to get a delivery agent by ID from storage.
///
/// ## Purpose
/// Used to retrieve a delivery agent by their ID.
///
/// ## Contents
/// - `delivery_id`: The ID of the delivery agent to retrieve.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Option<DeliveryDTO>")]
pub struct GetDelivery {
    pub delivery_id: String,
}

/// Message to get an order by ID from storage.
///
/// ## Purpose
/// Used to retrieve an order by its ID.
///
/// ## Contents
/// - `order_id`: The ID of the order to retrieve.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Option<OrderDTO>")]
pub struct GetOrder {
    pub order_id: u64,
}

/// Message to remove a client from storage.
///
/// ## Purpose
/// Used to record the removal of a client.
///
/// ## Contents
/// - `client_id`: The ID of the client to remove.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RemoveClient {
    pub client_id: String,
}

/// Message to remove a restaurant from storage.
///
/// ## Purpose
/// Used to record the removal of a restaurant.
///
/// ## Contents
/// - `restaurant_id`: The ID of the restaurant to remove.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RemoveRestaurant {
    pub restaurant_id: String,
}

/// Message to remove a delivery agent from storage.
///
/// ## Purpose
/// Used to record the removal of a delivery agent.
///
/// ## Contents
/// - `delivery_id`: The ID of the delivery agent to remove.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RemoveDelivery {
    pub delivery_id: String,
}

/// Message to remove an order from storage.
///
/// ## Purpose
/// Used to record the removal of an order.
///
/// ## Contents
/// - `order`: The [`OrderDTO`] representing the order to remove.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RemoveOrder {
    pub order: OrderDTO,
}

/// Message to add an authorized order to a restaurant.
///
/// ## Purpose
/// Used to record that an order has been authorized for a restaurant.
///
/// ## Contents
/// - `restaurant_id`: The ID of the restaurant.
/// - `order`: The [`OrderDTO`] representing the order.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AddAuthorizedOrderToRestaurant {
    pub restaurant_id: String,
    pub order: OrderDTO,
}

/// Message to add a pending order to a restaurant.
///
/// ## Purpose
/// Used to record that an order is pending for a restaurant.
///
/// ## Contents
/// - `restaurant_id`: The ID of the restaurant.
/// - `order`: The [`OrderDTO`] representing the order.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AddPendingOrderToRestaurant {
    pub restaurant_id: String,
    pub order: OrderDTO,
}

/// Message to remove an authorized order from a restaurant.
///
/// ## Purpose
/// Used to record the removal of an authorized order from a restaurant.
///
/// ## Contents
/// - `restaurant_id`: The ID of the restaurant.
/// - `order`: The [`OrderDTO`] representing the order.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RemoveAuthorizedOrderToRestaurant {
    pub restaurant_id: String,
    pub order: OrderDTO,
}

/// Message to remove a pending order from a restaurant.
///
/// ## Purpose
/// Used to record the removal of a pending order from a restaurant.
///
/// ## Contents
/// - `restaurant_id`: The ID of the restaurant.
/// - `order`: The [`OrderDTO`] representing the order.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RemovePendingOrderToRestaurant {
    pub restaurant_id: String,
    pub order: OrderDTO,
}

/// Message to get all restaurants from storage.
///
/// ## Purpose
/// Used to retrieve all restaurants.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Vec<RestaurantDTO>")]
pub struct GetRestaurants;

/// Message to get all restaurant info from storage.
///
/// ## Purpose
/// Used to retrieve information about all restaurants.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Vec<RestaurantInfo>")]
pub struct GetAllRestaurantsInfo;

/// Message to set the position of a delivery agent.
///
/// ## Purpose
/// Used to update the position of a delivery agent.
///
/// ## Contents
/// - `delivery_id`: The ID of the delivery agent.
/// - `position`: The (x, y) position.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetDeliveryPosition {
    pub delivery_id: String,
    pub position: (f32, f32),
}

/// Message to set the current client assigned to a delivery agent.
///
/// ## Purpose
/// Used to assign a client to a delivery agent.
///
/// ## Contents
/// - `delivery_id`: The ID of the delivery agent.
/// - `client_id`: The ID of the client.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetCurrentClientToDelivery {
    pub delivery_id: String,
    pub client_id: String,
}

/// Message to set the current order assigned to a delivery agent.
///
/// ## Purpose
/// Used to assign an order to a delivery agent.
///
/// ## Contents
/// - `delivery_id`: The ID of the delivery agent.
/// - `order`: The [`OrderDTO`] representing the order.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetCurrentOrderToDelivery {
    pub delivery_id: String,
    pub order: OrderDTO,
}

/// Message to set the status of a delivery agent.
///
/// ## Purpose
/// Used to update the status of a delivery agent.
///
/// ## Contents
/// - `delivery_id`: The ID of the delivery agent.
/// - `delivery_status`: The [`DeliveryStatus`] to set.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetDeliveryStatus {
    pub delivery_id: String,
    pub delivery_status: DeliveryStatus,
}

/// Message to set the delivery agent assigned to an order.
///
/// ## Purpose
/// Used to assign a delivery agent to an order.
///
/// ## Contents
/// - `order`: The [`OrderDTO`] representing the order.
/// - `delivery_id`: The ID of the delivery agent.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetDeliveryToOrder {
    pub order: OrderDTO,
    pub delivery_id: String,
}

/// Message to get all deliveries from storage.
///
/// ## Purpose
/// Used to retrieve all delivery agents.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Vec<DeliveryDTO>")]
pub struct GetDeliveries;

/// Message to get all available deliveries from storage.
///
/// ## Purpose
/// Used to retrieve all available delivery agents.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Vec<DeliveryDTO>")]
pub struct GetAllAvailableDeliveries;

/// Message to set the status of an order.
///
/// ## Purpose
/// Used to update the status of an order.
///
/// ## Contents
/// - `order`: The [`OrderDTO`] representing the order.
/// - `order_status`: The [`OrderStatus`] to set.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetOrderStatus {
    pub order: OrderDTO,
    pub order_status: OrderStatus,
}

/// Message struct used to set the expected time for an order.
///
/// ## Purpose
/// Used to update the expected delivery time of an order.
/// 
/// # Fields
/// - `order_id`: The unique identifier of the order.
/// - `expected_time`: Expected time to deliver the order.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetOrderExpectedTime {
    pub order_id: u64,
    pub expected_time: u64,
} 

/////////////////////////////////////////////////////////////////////
// Mensajes de servicios internos
/////////////////////////////////////////////////////////////////////

/// Message to insert an accepted delivery for an order.
///
/// ## Purpose
/// Used to record that a delivery agent has accepted an order.
///
/// ## Contents
/// - `order_id`: The ID of the order.
/// - `delivery_id`: The ID of the delivery agent.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct InsertAcceptedDelivery {
    pub order_id: u64,
    pub delivery_id: String,
}

/// Message to remove accepted deliveries for an order.
///
/// ## Purpose
/// Used to remove all accepted deliveries for a specific order.
///
/// ## Contents
/// - `order_id`: The ID of the order.
///   Returns: An optional set of delivery IDs that were removed.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Option<HashSet<String>>")]
pub struct RemoveAcceptedDeliveries {
    pub order_id: u64,
}
