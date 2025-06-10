use actix::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr};

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
