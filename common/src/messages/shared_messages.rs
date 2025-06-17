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

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[serde(tag = "type")]
#[rtype(result = "()")]
pub enum NetworkMessage {
    // All Users messages
    WhoIsLeader(WhoIsLeader),
    LeaderIs(LeaderIs),
    RegisterUser(RegisterUser),
    RecoveredInfo(UserDTO),
    NoRecoveredInfo,

    // Client messages
    RequestThisOrder(RequestThisOrder),
    RequestNearbyRestaurants(RequestNearbyRestaurants),
    OrderFinalized(OrderFinalized),
    DeliveryExpectedTime(DeliveryExpectedTime),

    // Delivery messages
    IAmAvailable(IAmAvailable),
    AcceptedOrder(AcceptedOrder),
    OrderDelivered(OrderDelivered),
    DeliverThisOrder(DeliverThisOrder),
    IAmDelivering(IAmDelivering),

    // Payment messages
    RequestAuthorization(RequestAuthorization),
    AuthorizationResult(AuthorizationResult),
    PaymentCompleted(PaymentCompleted),
    BillPayment(BillPayment),

    // Restaurant messages
    NewOrder(NewOrder),
    UpdateOrderStatus(UpdateOrderStatus),
    CancelOrder(CancelOrder),
    RequestNearbyDelivery(RequestNearbyDelivery),
    DeliveryAccepted(DeliveryAccepted),
    NearbyDeliveries(NearbyDeliveries),
    DeliveryAvailable(DeliveryAvailable),

    // Coordinator messages
    NearbyRestaurants(NearbyRestaurants),
    NotifyOrderUpdated(NotifyOrderUpdated),
    NewOfferToDeliver(NewOfferToDeliver),
    DeliveryNoNeeded(DeliveryNoNeeded),

    // CoordinatorManager messages
    RequestNewStorageUpdates(RequestNewStorageUpdates),
    StorageUpdates(StorageUpdates),
    RequestAllStorage(RequestAllStorage),
    RecoverStorageOperations(RecoverStorageOperations),
    LeaderElection(LeaderElection),
    Ping(Ping),
    Pong(Pong),

    RetryLater(RetryLater),
}

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct WhoIsLeader {
    pub origin_addr: SocketAddr,
    pub user_id: String,
}

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct RegisterUser {
    pub origin_addr: SocketAddr,
    pub user_id: String,
    pub position: (f32, f32),
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RecoverProcedure {
    pub user_info: UserDTO,
}

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub struct RetryLater {
    pub origin_addr: SocketAddr,
}
