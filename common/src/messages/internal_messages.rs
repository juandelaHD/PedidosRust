use crate::messages::AcceptedOrder;
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

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct ApplyStorageUpdates {
    pub is_leader: bool,
    pub updates: Vec<(u64, StorageLogMessage)>,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetStorageUpdatesLog {
    pub updates_log: HashMap<u64, StorageLogMessage>,
}

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

    InsertAcceptedDelivery(InsertAcceptedDelivery),
    RemoveAcceptedDeliveries(RemoveAcceptedDeliveries),
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AddClient {
    pub client: ClientDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AddRestaurant {
    pub restaurant: RestaurantDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AddDelivery {
    pub delivery: DeliveryDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AddOrder {
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Option<ClientDTO>")]
pub struct GetClient {
    pub client_id: String,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Option<RestaurantDTO>")]
pub struct GetRestaurant {
    pub restaurant_id: String,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Option<DeliveryDTO>")]
pub struct GetDelivery {
    pub delivery_id: String,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Option<OrderDTO>")]
pub struct GetOrder {
    pub order_id: u64,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RemoveClient {
    pub client_id: String,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RemoveRestaurant {
    pub restaurant_id: String,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RemoveDelivery {
    pub delivery_id: String,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RemoveOrder {
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AddAuthorizedOrderToRestaurant {
    pub restaurant_id: String,
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AddPendingOrderToRestaurant {
    pub restaurant_id: String,
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RemoveAuthorizedOrderToRestaurant {
    pub restaurant_id: String,
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RemovePendingOrderToRestaurant {
    pub restaurant_id: String,
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Vec<RestaurantDTO>")]
pub struct GetRestaurants;

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Vec<RestaurantInfo>")]
pub struct GetAllRestaurantsInfo;

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetDeliveryPosition {
    pub delivery_id: String,
    pub position: (f32, f32),
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetCurrentClientToDelivery {
    pub delivery_id: String,
    pub client_id: String,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetCurrentOrderToDelivery {
    pub delivery_id: String,
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetDeliveryStatus {
    pub delivery_id: String,
    pub delivery_status: DeliveryStatus,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetDeliveryToOrder {
    pub order: OrderDTO,
    pub delivery_id: String,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Vec<DeliveryDTO>")]
pub struct GetDeliveries;

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Vec<DeliveryDTO>")]
pub struct GetAllAvailableDeliveries;

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetOrderStatus {
    pub order: OrderDTO,
    pub order_status: OrderStatus,
}

/////////////////////////////////////////////////////////////////////
// Mensajes de servicios internos
/////////////////////////////////////////////////////////////////////

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct InsertAcceptedDelivery {
    pub order_id: u64,
    pub delivery_id: String,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Option<HashSet<String>>")]
pub struct RemoveAcceptedDeliveries {
    pub order_id: u64,
}
