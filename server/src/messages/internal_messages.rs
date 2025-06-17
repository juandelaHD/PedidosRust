use crate::server_actors::coordinator::Coordinator;
use crate::server_actors::coordinator_manager::CoordinatorManager;
use crate::server_actors::services::orders_services::OrderService;
use crate::server_actors::storage::Storage;
use actix::prelude::*;
use common::messages::internal_messages::StorageLogMessage;
use common::network::communicator::Communicator;
use common::types::dtos::{DeliveryDTO, OrderDTO, Snapshot};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;

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

//___________________________________________________________________
#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct AddOrderAccepted {
    pub order: OrderDTO,
    pub delivery: DeliveryDTO,
    pub addr: Addr<OrderService>,
}

#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct FinishDeliveryAssignment {
    pub order: OrderDTO,
    pub addr: Addr<OrderService>,
}
// ____________________________________________________________________

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
pub struct SetActorsAddresses {
    pub coordinator_addr: Addr<Coordinator>,
    pub storage_addr: Addr<Storage>,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "u64")]
pub struct GetMinLogIndex;

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "HashMap<u64, StorageLogMessage>")]
pub struct GetLogsFromIndex {
    pub index: u64,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Snapshot")]
pub struct GetAllStorage;
