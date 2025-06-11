use crate::messages::client_messages::*;
use crate::types::dtos::UserDTO;
use actix::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::TcpStream;

use crate::messages::client_messages::*;
use crate::messages::coordinator_messages::*;
use crate::messages::coordinatormanager_messages::*;
use crate::messages::delivery_messages::*;
use crate::messages::payment_messages::*;
use crate::messages::restaurant_messages::*;

#[derive(Serialize, Deserialize, Debug, Message)]
#[serde(tag = "type")]
#[rtype(result = "()")]
pub enum NetworkMessage {
    // All Users messages
    WhoIsLeader(WhoIsLeader),
    LeaderIs(LeaderIs),
    RegisterUser(RegisterUser),
    RecoveredInfo(Option<UserDTO>),

    // Client messages
    RequestThisOrder(RequestThisOrder),
    RequestNearbyRestaurants(RequestNearbyRestaurants),
    OrderFinalized(OrderFinalized),

    // Delivery messages
    IAmAvailable(IAmAvailable),
    AcceptOrder(AcceptOrder),
    OrderDelivered(OrderDelivered),

    // Payment messages

    // Restaurant messages
    NewOrder(NewOrder),
    UpdateOrderStatus(UpdateOrderStatus),
    CancelOrder(CancelOrder),
    OrderIsPreparing(OrderIsPreparing),
    RequestDelivery(RequestDelivery),
    DeliverThisOrder(DeliverThisOrder),

    // Coordinator messages
    AuthorizationResult(AuthorizationResult),
    NearbyRestaurants(NearbyRestaurants),
    NotifyOrderUpdated(NotifyOrderUpdated),

    // CoordinatorManager messages
    RequestNewStorageUpdates(RequestNewStorageUpdates),
    StorageUpdates(StorageUpdates),
    RequestAllStorage(RequestAllStorage),
    RecoverStorageOperations(RecoverStorageOperations),
    LeaderElection(LeaderElection),
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[rtype(result = "()")]
pub struct WhoIsLeader {
    pub origin_addr: SocketAddr,
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[rtype(result = "()")]
pub struct LeaderIs {
    pub coord_addr: SocketAddr,
}

// TODO: Borrar, quedó viejo
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct StartRunning;

#[derive(Message)]
#[rtype(result = "()")]
pub struct NewLeaderConnection {
    pub addr: SocketAddr,
    pub stream: TcpStream,
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[rtype(result = "()")]
pub struct RegisterUser {
    pub origin_addr: SocketAddr,
    pub user_id: String,
}
