//use crate::Chef;
use crate::server_acceptor::acceptor::Acceptor;
use actix::Message;
//use common::types::order::OrderDTO;
use crate::server_actors::coordinator::Coordinator;
use crate::server_actors::coordinator_manager::CoordinatorManager;
use actix::Addr;
use common::network::communicator::Communicator;
use common::types::delivery_status::DeliveryStatus;
use common::types::dtos::ClientDTO;
use common::types::dtos::DeliveryDTO;
use common::types::dtos::OrderDTO;
use common::types::dtos::RestaurantDTO;
use common::types::order_status::OrderStatus;
use common::types::restaurant_info::RestaurantInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

/////////////////////////////////////////////////////////////////////
// Mensajes del Aceptador al Coordinator
/////////////////////////////////////////////////////////////////////
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct RegisterConnection {
    pub client_addr: SocketAddr,
    pub communicator: Communicator<Coordinator>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SetCoordinatorManager {
    pub addr: Addr<CoordinatorManager>,
}

/////////////////////////////////////////////////////////////////////
// Mensajes del Aceptador al Coordinator Manager
/////////////////////////////////////////////////////////////////////
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct RegisterConnectionWithCoordinator {
    pub remote_addr: SocketAddr,
    pub communicator: Communicator<Coordinator>,
}

/////////////////////////////////////////////////////////////////////
// Mensajes del storage
/////////////////////////////////////////////////////////////////////

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct ApplyStorageUpdates {
    pub updates: HashMap<u64, StorageLogMessage>,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SetStorageUpdatesLog {
    pub updates_log: HashMap<u64, StorageLogMessage>,
}

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[serde(tag = "type")]
#[rtype(result = "()")]
pub enum StorageLogMessage {
    AddClient(AddClient),
    AddRestaurant(AddRestaurant),
    AddDelivery(AddDelivery),
    RemoveClient(RemoveClient),
    RemoveRestaurant(RemoveRestaurant),
    RemoveDelivery(RemoveDelivery),
    GetRestaurants(GetRestaurants),
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
    // SetOrderToClient(SetOrderToClient),
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
    pub restaurant_id: String,
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
    pub order_id: u64,
}

// #[derive(Message, Debug, Clone, Serialize, Deserialize)]
// #[rtype(result = "()")]
// pub struct SetOrderToClient {
//     pub client_id: String,
//     pub order_id: u64,
// }

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
// Mensajes del Order Service
/////////////////////////////////////////////////////////////////////

#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct SetCoordinatorAddr {
    pub coordinator_addr: Addr<Coordinator>,
}

/////////////////////////////////////////////////////////////////////
// Mesnajes del comunicador
/////////////////////////////////////////////////////////////////////

// #[derive(Message, Debug, Clone, Serialize, Deserialize)]
// #[rtype(result = "()")]
// pub struct ForwardMessage {
//     pub addr: SocketAddr,
//     pub message: Message,
// }

// #[derive(Message, Debug, Clone, Serialize, Deserialize)]
// #[rtype(result = "()")]
// pub struct SendToSocket {
//     pub message: Message,
// }

/////////////////////////////////////////////////////////////////////
// Mensajes de servicios internos
/////////////////////////////////////////////////////////////////////

/////////////////////////////////////////////////////////////////////
// Men
/////////////////////////////////////////////////////////////////////
