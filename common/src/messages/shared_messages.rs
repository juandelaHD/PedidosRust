use actix::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr};
use tokio::net::TcpStream;
use crate::messages::client_messages::*;
use crate::types::dtos::UserDTO;

use crate::messages::client_messages::*;
use crate::messages::delivery_messages::*;
use crate::messages::payment_messages::*;
use crate::messages::restaurant_messages::*;
use crate::messages::coordinator_messages::*;
use crate::messages::coordinatormanager_messages::*;


#[derive(Serialize, Deserialize, Message)]
#[serde(tag = "type")]
#[rtype(result = "()")]
pub enum NetworkMessage {
    WhoIsLeader(WhoIsLeader),
    LeaderIs(LeaderIs),
    RequestNewStorageUpdates(RequestNewStorageUpdates),
    StorageUpdates(StorageUpdates),
    RequestAllStorage(RequestAllStorage),
    RecoverStorageOperations(RecoverStorageOperations),
    LeaderElection(LeaderElection),
    RegisterUser(RegisterUser),
    RecoveredInfo(Option<UserDTO>),

    // Client messages
    RequestNearbyRestaurants(RequestNearbyRestaurants),
    RequestThisOrder(RequestThisOrder),

    // Delivery messages

    // Payment messages

    // Restaurant messages

    // Coordinator messages

    // CoordinatorManager messages
}

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub struct WhoIsLeader {
    pub origin_addr: SocketAddr,
}

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub struct LeaderIs {
    pub coord_addr: SocketAddr,
}

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub struct RequestNewStorageUpdates {
    pub start_index: u64,
}

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub struct StorageUpdates {
    pub updates: HashMap<u64, String>,
}

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub struct RequestAllStorage;

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub struct RecoverStorageOperations {
    pub storage_recover_msg_list: HashMap<u64, String>,
    pub current_msg_log: HashMap<u64, String>,
}

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub struct LeaderElection {
    pub candidates: Vec<String>,
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

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub struct RegisterUser {
    pub origin_addr: SocketAddr,
    pub user_id: String,
}
