use actix::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct RequestNewStorageUpdates {
    pub start_index: u64,
}

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct StorageUpdates {
    pub updates: HashMap<u64, String>,
}

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct RequestAllStorage;

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct RecoverStorageOperations {
    pub storage_recover_msg_list: HashMap<u64, String>,
    pub current_msg_log: HashMap<u64, String>,
}

#[derive(Serialize, Deserialize, Debug, Message)]
#[rtype(result = "()")]
pub struct SetStorageUpdatesLog {
    pub storage_recover_msg_list: HashMap<u64, String>,
    pub current_msg_log: HashMap<u64, String>,
}

/////////////////////////////////////////////////////////////////////
// Mensajes del Coordinator Manager
/////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct Ping {
    pub from: SocketAddr,
}

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct Pong {
    pub from: SocketAddr,
}

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct LeaderElection {
    pub initiator: SocketAddr,
    pub candidates: Vec<SocketAddr>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct CheckPongTimeout;
