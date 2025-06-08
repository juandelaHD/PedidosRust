use std::{collections::HashMap, net::SocketAddr};

use actix::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Message)]
#[serde(tag = "type")]
#[rtype(result = "()")]
pub enum NetworkMessage {
    // Agregar todos los mensajes que se envían a través de la red
    WhoIsLeader,
    LeaderIs {
        addr: SocketAddr,
    },
    RequestNewStorageUpdates {
        start_index: u64,
    },
    StorageUpdates {
        updates: HashMap<u64, String>,
    },
    RequestAllStorage,
    RecoverStorageOperations {
        storage_recover_msg_list: HashMap<u64, String>,
        current_msg_log: HashMap<u64, String>,
    },
    LeaderElection {
        candidates: Vec<String>,
    },
}

// TODO: Borrar, quedó viejo
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct StartRunning;
