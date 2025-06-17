use crate::messages::StorageLogMessage;
use crate::types::dtos::Snapshot;
use actix::Message;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct RequestNewStorageUpdates {
    pub coordinator_id: String,
    pub start_index: u64,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct StorageUpdates {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub updates: HashMap<u64, StorageLogMessage>,
}

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct RequestAllStorage {
    pub coordinator_id: String,
}

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct StorageSnapshot {
    pub snapshot: Snapshot,
}

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
    pub initiator: String,
    pub candidates: Vec<String>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct CheckPongTimeout;
