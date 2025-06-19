use crate::messages::client_messages::*;
use crate::messages::coordinator_messages::*;
use crate::messages::coordinatormanager_messages::*;
use crate::messages::delivery_messages::*;
use crate::messages::payment_messages::*;
use crate::messages::restaurant_messages::*;
use crate::types::dtos::UserDTO;
use actix::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::TcpStream;

/// Enum representing all possible network messages exchanged between system components.
///
/// # Purpose
/// This enum encapsulates all message types that can be sent over the network between clients,
/// coordinators, restaurants, delivery agents, payment gateways, and coordinator managers.
///
/// # Contents
/// Each variant wraps a specific message struct, grouping messages by their origin or target.
/// See the documentation for each variant's struct for details.
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[serde(tag = "type")]
#[rtype(result = "()")]
pub enum NetworkMessage {
    // Users messages
    /// Query for the current leader in the system.
    WhoIsLeader(WhoIsLeader),
    /// Response indicating the current leader's address.
    LeaderIs(LeaderIs),
    /// Response indicating the current leader's ID.
    LeaderIdIs(LeaderIdIs),
    /// Register a new user in the system.
    RegisterUser(RegisterUser),
    /// Response with recovered user information.
    RecoveredInfo(UserDTO),
    /// Indicates no recovered information is available.
    NoRecoveredInfo,

    // Client messages
    /// Client requests to place a new order.
    RequestThisOrder(RequestThisOrder),
    /// Client requests a list of nearby restaurants.
    RequestNearbyRestaurants(RequestNearbyRestaurants),
    /// Notifies the client that their order has been finalized.
    OrderFinalized(OrderFinalized),
    /// Informs the client of the expected delivery time.
    DeliveryExpectedTime(DeliveryExpectedTime),

    // Delivery messages
    /// Delivery agent announces availability.
    IAmAvailable(IAmAvailable),
    /// Delivery agent accepts an order.
    AcceptedOrder(AcceptedOrder),
    /// Delivery agent notifies that an order has been delivered.
    OrderDelivered(OrderDelivered),
    /// Assigns a delivery agent to deliver an order.
    DeliverThisOrder(DeliverThisOrder),
    /// Delivery agent updates about ongoing delivery.
    IAmDelivering(IAmDelivering),

    // Payment messages
    /// Requests payment authorization for an order.
    RequestAuthorization(RequestAuthorization),
    /// Result of a payment authorization request.
    AuthorizationResult(AuthorizationResult),
    /// Notifies that payment has been completed.
    PaymentCompleted(PaymentCompleted),
    /// Requests billing for a payment.
    BillPayment(BillPayment),

    // Restaurant messages
    /// Notifies a restaurant of a new order.
    NewOrder(NewOrder),
    /// Updates the status of an order at a restaurant.
    UpdateOrderStatus(UpdateOrderStatus),
    /// Cancels an order at a restaurant.
    CancelOrder(CancelOrder),
    /// Restaurant requests nearby delivery agents.
    RequestNearbyDelivery(RequestNearbyDelivery),
    /// Notifies that a delivery agent has accepted a delivery.
    DeliveryAccepted(DeliveryAccepted),
    /// Provides a list of nearby delivery agents.
    NearbyDeliveries(NearbyDeliveries),
    /// Notifies a restaurant that a delivery agent is available.
    DeliveryAvailable(DeliveryAvailable),

    // Coordinator messages
    /// Provides a client with a list of nearby restaurants.
    NearbyRestaurants(NearbyRestaurants),
    /// Notifies a peer that an order has been updated.
    NotifyOrderUpdated(NotifyOrderUpdated),
    /// Offers a delivery agent a new order to deliver.
    NewOfferToDeliver(NewOfferToDeliver),
    /// Notifies a delivery agent that their services are not needed.
    DeliveryNoNeeded(DeliveryNoNeeded),

    // CoordinatorManager messages
    /// Requests new storage updates from the coordinator manager.
    RequestNewStorageUpdates(RequestNewStorageUpdates),
    /// Provides storage updates.
    StorageUpdates(StorageUpdates),
    /// Requests all storage data.
    RequestAllStorage(RequestAllStorage),
    /// Provides a snapshot of storage.
    StorageSnapshot(StorageSnapshot),
    /// Requests recovery of storage operations.
    RecoverStorageOperations(RecoverStorageOperations),
    /// Initiates or participates in a leader election.
    LeaderElection(LeaderElection),
    /// Ping message for liveness checks.
    Ping(Ping),
    /// Pong response for liveness checks.
    Pong(Pong),

    /// Requests to retry an operation later.
    RetryLater(RetryLater),

    /// Notifies that a TCP connection has been closed.
    ConnectionClosed(ConnectionClosed),
}

/// Message sent to query for the current leader in the system.
///
/// ## Purpose
/// Used by a node to discover the current leader's address and user ID.
///
/// # Contents
/// - `origin_addr`: The address of the querying node.
/// - `user_id`: The ID of the querying user.
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct WhoIsLeader {
    pub origin_addr: SocketAddr,
    pub user_id: String,
}

/// Message sent to inform a node of the current leader's address.
///
/// ## Purpose
/// Used as a response to `WhoIsLeader`.
///
/// ## Contents
/// - `coord_addr`: The address of the current leader.
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct LeaderIs {
    pub coord_addr: SocketAddr,
}

/// Message sent to inform a node of the current leader's user ID.
///
/// ## Purpose
/// Used as a response to `WhoIsLeader`.
///
/// ## Contents
/// - `leader_id`: The user ID of the current leader.
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct LeaderIdIs {
    pub leader_id: String,
}

/// Message sent to start the running of the system.
///
/// ## Purpose
/// Used to initiate the operations after all components are initialized.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct StartRunning;

/// Message sent to notify a new leader connection.
///
/// ## Purpose
/// Used to inform the system about a new leader connection established.
///
/// ## Contents
/// - `addr`: The address of the new leader.
/// - `stream`: The TCP stream for the new leader connection.
#[derive(Message)]
#[rtype(result = "()")]
pub struct NewLeaderConnection {
    pub addr: SocketAddr,
    pub stream: TcpStream,
}

/// Message sent to register a new user in the system.
///
/// ## Purpose
/// Used by a node to register itself with the system.
///
/// ## Contents
/// - `origin_addr`: The address of the registering node.
/// - `user_id`: The ID of the user.
/// - `position`: The (x, y) position of the user.
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct RegisterUser {
    pub origin_addr: SocketAddr,
    pub user_id: String,
    pub position: (f32, f32),
}

/// Message sent to recover user information.
///
/// ## Purpose
/// Used to request recovery of user information, typically after a crash or restart.
///
/// ## Contents
/// - `user_info`: The user information to be recovered, encapsulated in a `UserDTO`.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RecoverProcedure {
    pub user_info: UserDTO,
}

/// Message sent to request a retry of an operation at a later time.
///
/// ## Purpose
/// Used to indicate that an operation should be retried later, typically due to temporary unavailability.
///
/// ## Contents
/// - `origin_addr`: The address of the node that should retry.
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct RetryLater {
    pub origin_addr: SocketAddr,
}

/// Message sent to notify that a TCP connection has been closed.
///
/// ## Purpose
/// Used to inform the system that a connection to a remote address has been closed.
///
/// ## Contents
/// - `remote_addr`: The address of the remote peer whose connection was closed.
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct ConnectionClosed {
    pub remote_addr: SocketAddr,
}

/////////////////////////////////////////////////////////////////////
///// Mensajes del communicator
/// /////////////////////////////////////////////////////////////////////
#[derive(Message)]
#[rtype(result = "()")]
pub struct Shutdown;


/// Mensaje compartido entre usuarios
/// Mensaje interno para iniciar el flujo después del delay
pub struct StartRunningMsg;

impl Message for StartRunningMsg {
    type Result = ();
}