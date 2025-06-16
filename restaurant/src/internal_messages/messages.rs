use crate::restaurant_actors::{chef::Chef, restaurant::Restaurant};
use actix::{Addr, Message};
use common::network::communicator::Communicator;
use common::types::dtos::OrderDTO;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SendToKitchen {
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AssignToChef {
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct SendThisOrder {
    pub order: OrderDTO,
}

#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct IAmAvailable {
    pub chef_addr: Addr<Chef>,
    pub order: OrderDTO,
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct ShareCommunicator {
    pub communicator: Arc<Communicator<Restaurant>>,
}
