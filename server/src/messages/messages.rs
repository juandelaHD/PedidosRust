//use crate::Chef;
use actix::Message;
//use common::types::order::OrderDTO;
use common::types::delivery_status::DeliveryStatus;
use common::types::dtos::ClientDTO;
use common::types::dtos::DeliveryDTO;
use common::types::dtos::OrderDTO;
use common::types::dtos::RestaurantDTO;
use common::types::order_status::OrderStatus;
use serde::{Deserialize, Serialize};

/////////////////////////////////////////////////////////////////////
// Mensajes del storage
/////////////////////////////////////////////////////////////////////
///
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ApplyStorageUpdates {
    pub updates: HashMap<u64, Message>,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetStorageUpdatesLog {
    pub updates_log: HashMap<u64, Message>,
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
    pub order_id: u64,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetOrderToClient {
    pub client_id: String,
    pub order_id: u64,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AddAuthorizedOrderToRestaurant {
    pub restaurant_id: String,
    pub order_id: u64,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AddPendingOrderToRestaurant {
    pub restaurant_id: String,
    pub order_id: u64,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RemoveAuthorizedOrderToRestaurant {
    pub restaurant_id: String,
    pub order_id: u64,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RemovePendingOrderToRestaurant {
    pub restaurant_id: String,
    pub order_id: u64,
}

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
    pub order_id: u64,
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
    pub order_id: u64,
    pub delivert_id: string,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetOrderStatus {
    pub order_id: u64,
    pub order_status: OrderStatus,
}

/////////////////////////////////////////////////////////////////////
// Mensajes del Reaper
/////////////////////////////////////////////////////////////////////

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct StartReapProcess {
    pub user_id: String,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct CheckReapUser {
    pub user_id: String,
}

/////////////////////////////////////////////////////////////////////
// Mesnajes del comunicador
/////////////////////////////////////////////////////////////////////

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ForwardMessage {
    pub addr: SocketAddr,
    pub message: Message,
}

// #[derive(Message, Debug, Clone, Serialize, Deserialize)]
// #[rtype(result = "()")]
// pub struct SendToSocket {
//     pub message: Message,
// }

/////////////////////////////////////////////////////////////////////
// Mensajes de servicios internos
/////////////////////////////////////////////////////////////////////

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RequestNearbyRestaurants {
    pub client_dto: ClientDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct NearbyRestaurants {
    pub nearby_restaurants: Vec<RestaurantDTO>,
}

/////////////////////////////////////////////////////////////////////
// Men
/////////////////////////////////////////////////////////////////////
