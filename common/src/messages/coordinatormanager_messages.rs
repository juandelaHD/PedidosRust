use crate::messages::StorageLogMessage;
use crate::types::dtos::Snapshot;
use actix::Message;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::HashMap;
use std::net::SocketAddr;

/// Message sent to request new storage updates from the coordinator manager.
///
/// ## Purpose
/// Used by a coordinator to request updates to the storage log starting from a specific index.
///
/// ## Contents
/// - `coordinator_id`: The ID of the requesting coordinator.
/// - `start_index`: The index from which to start sending updates.
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct RequestNewStorageUpdates {
    pub coordinator_id: String,
    pub start_index: u64,
}

/// Message sent to provide storage updates to a coordinator.
///
/// ## Purpose
/// Used by the coordinator manager to send a batch of storage log updates.
///
/// ## Contents
/// - `updates`: A map of log indices to [`StorageLogMessage`]s.
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct StorageUpdates {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub updates: HashMap<u64, StorageLogMessage>,
}

/// Message sent to request all storage data from the coordinator manager.
///
/// ## Purpose
/// Used by a coordinator to request a full snapshot of the storage.
///
/// ## Contents
/// - `coordinator_id`: The ID of the requesting coordinator.
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct RequestAllStorage {
    pub coordinator_id: String,
}

/// Message sent to provide a snapshot of storage.
///
/// ## Purpose
/// Used by the coordinator manager to send a full snapshot of the storage state.
///
/// ## Contents
/// - `snapshot`: The [`Snapshot`] representing the current storage state.
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct StorageSnapshot {
    pub snapshot: Snapshot,
}

/// Message sent to request recovery of storage operations.
///
/// ## Purpose
/// Used by a coordinator to request recovery of storage operations from the coordinator manager.
///
/// ## Contents
/// - `storage_recover_msg_list`: A map of log indices to recovery messages.
/// - `current_msg_log`: A map of log indices to current messages.
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct RecoverStorageOperations {
    pub storage_recover_msg_list: HashMap<u64, String>,
    pub current_msg_log: HashMap<u64, String>,
}

/// Message sent to set the storage updates log.
///
/// ## Purpose
/// Used internally to update the storage log state.
///
/// ## Contents
/// - `storage_recover_msg_list`: A map of log indices to recovery messages.
/// - `current_msg_log`: A map of log indices to current messages.
#[derive(Serialize, Deserialize, Debug, Message)]
#[rtype(result = "()")]
pub struct SetStorageUpdatesLog {
    pub storage_recover_msg_list: HashMap<u64, String>,
    pub current_msg_log: HashMap<u64, String>,
}

/////////////////////////////////////////////////////////////////////
// Mensajes del Coordinator Manager
/////////////////////////////////////////////////////////////////////

/// Message sent as a ping for liveness checks.
///
/// ## Purpose
/// Used to check if a node is alive.
///
/// ## Contents
/// - `from`: The address of the node sending the ping.
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct Ping {
    pub from: SocketAddr,
}

/// Message sent as a pong response for liveness checks.
///
/// ## Purpose
/// Used to respond to a ping message.
///
/// ## Contents
/// - `from`: The address of the node sending the pong.
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct Pong {
    pub from: SocketAddr,
}

/// Message sent to initiate or participate in a leader election.
///
/// ## Purpose
/// Used by coordinators to elect a new leader among themselves.
///
/// ## Contents
/// - `initiator`: The ID of the node initiating the election.
/// - `candidates`: A list of candidate node IDs.
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct LeaderElection {
    pub initiator: String,
    pub candidates: Vec<String>,
}

/// Message sent to check for pong timeout (internal use).
#[derive(Message)]
#[rtype(result = "()")]
pub struct CheckPongTimeout;
