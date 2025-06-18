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

/// Message sent by the acceptor to register a new client connection with the coordinator.
///
/// ## Purpose
/// Registers a new client and its communicator with the coordinator actor.
///
/// ## Contents
/// - `client_addr`: The socket address of the client.
/// - `communicator`: The [`Communicator`] for the client connection.
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct RegisterConnection {
    pub client_addr: SocketAddr,
    pub communicator: Communicator<Coordinator>,
}

/// Message sent to set the coordinator manager address in the coordinator.
///
/// ## Purpose
/// Allows the coordinator to know the address of the coordinator manager actor.
///
/// ## Contents
/// - `addr`: The [`Addr<CoordinatorManager>`] to set.
#[derive(Message)]
#[rtype(result = "()")]
pub struct SetCoordinatorManager {
    pub addr: Addr<CoordinatorManager>,
}

/////////////////////////////////////////////////////////////////////
// Mensajes del Aceptador al Coordinator Manager
/////////////////////////////////////////////////////////////////////

/// Message sent by the acceptor to register a new connection with the coordinator manager.
///
/// ## Purpose
/// Registers a new peer and its communicator with the coordinator manager actor.
///
/// ## Contents
/// - `remote_addr`: The socket address of the remote peer.
/// - `communicator`: The [`Communicator`] for the peer connection.
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct RegisterConnectionWithCoordinator {
    pub remote_addr: SocketAddr,
    pub communicator: Communicator<Coordinator>,
}

/// Message sent to add an accepted order and its delivery assignment.
///
/// ## Purpose
/// Used to record that an order has been accepted by a delivery agent and to notify the order service.
///
/// ## Contents
/// - `order`: The [`OrderDTO`] representing the order.
/// - `delivery`: The [`DeliveryDTO`] representing the delivery agent.
/// - `addr`: The [`Addr<OrderService>`] for the order service.
#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct AddOrderAccepted {
    pub order: OrderDTO,
    pub delivery: DeliveryDTO,
    pub addr: Addr<OrderService>,
}

/// Message sent to finish a delivery assignment for an order.
///
/// ## Purpose
/// Used to notify the order service that a delivery assignment has been completed.
///
/// ## Contents
/// - `order`: The [`OrderDTO`] representing the order.
/// - `addr`: The [`Addr<OrderService>`] for the order service.
#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct FinishDeliveryAssignment {
    pub order: OrderDTO,
    pub addr: Addr<OrderService>,
}

/////////////////////////////////////////////////////////////////////
// Mensajes del Reaper
/////////////////////////////////////////////////////////////////////

/// Message to start the reaping process for a user.
///
/// ## Purpose
/// Initiates the process to check and clean up resources for a user (e.g., after disconnect).
///
/// ## Contents
/// - `user_id`: The ID of the user to reap.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct StartReapProcess {
    pub user_id: String,
}

/// Message to check if a user should be reaped (cleaned up).
///
/// ## Purpose
/// Used to verify if a user is eligible for resource cleanup.
///
/// ## Contents
/// - `user_id`: The ID of the user to check.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct CheckReapUser {
    pub user_id: String,
}

/////////////////////////////////////////////////////////////////////
// Mensajes del Order Service
/////////////////////////////////////////////////////////////////////

/// Message to set the addresses of the coordinator and storage actors in the order service.
///
/// ## Purpose
/// Allows the order service to communicate with the coordinator and storage actors.
///
/// ## Contents
/// - `coordinator_addr`: The [`Addr<Coordinator>`] for the coordinator.
/// - `storage_addr`: The [`Addr<Storage>`] for the storage actor.
#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct SetActorsAddresses {
    pub coordinator_addr: Addr<Coordinator>,
    pub storage_addr: Addr<Storage>,
}

/// Message to get the minimum log index from storage.
///
/// ## Purpose
/// Requests the smallest log index currently stored.
///
/// ## Returns
/// - `u64`: The minimum log index.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "u64")]
pub struct GetMinLogIndex;

/// Message to get all storage log messages from a given index.
///
/// ## Purpose
/// Requests all storage log messages starting from a specific index.
///
/// ## Contents
/// - `index`: The starting log index.
///
/// ## Returns
/// - `HashMap<u64, StorageLogMessage>`: A map of log indices to storage log messages
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "HashMap<u64, StorageLogMessage>")]
pub struct GetLogsFromIndex {
    pub index: u64,
}

/// Message to get a full snapshot of all storage data.
///
/// ## Purpose
/// Requests a complete snapshot of the current storage state.
///
/// ## Returns
/// - [`Snapshot`]: The current storage snapshot.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "Snapshot")]
pub struct GetAllStorage;
