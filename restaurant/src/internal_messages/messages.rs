use crate::restaurant_actors::{chef::Chef, restaurant::Restaurant};
use actix::{Addr, Message};
use common::network::communicator::Communicator;
use common::types::dtos::OrderDTO;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Message sent from the restaurant to the kitchen to enqueue a new order for preparation.
///
/// Contains the [`OrderDTO`](../../common/types/dtos/struct.OrderDTO.html) to be prepared.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SendToKitchen {
    pub order: OrderDTO,
}

/// Message sent from the kitchen to a chef to assign an order for preparation.
///
/// Contains the [`OrderDTO`](../../common/types/dtos/struct.OrderDTO.html) to be cooked.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AssignToChef {
    pub order: OrderDTO,
}

/// Message sent from a chef to the delivery assigner when an order is ready for delivery.
///
/// Contains the [`OrderDTO`](../../common/types/dtos/struct.OrderDTO.html) that is ready.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SendThisOrder {
    pub order: OrderDTO,
}

/// Message sent from a chef to the kitchen indicating the chef is available for a new order.
///
/// Contains the address of the chef and the last order handled.
#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct IAmAvailable {
    pub chef_addr: Addr<Chef>,
    pub order: OrderDTO,
}

/// Message used to share the communicator instance between restaurant actors.
///
/// Contains an [`Arc`] to the [`Communicator`](../../common/network/communicator/struct.Communicator.html).
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct ShareCommunicator {
    pub communicator: Arc<Communicator<Restaurant>>,
}
