use crate::Chef;
use actix::Message;
use common::types::order::OrderDTO;
use serde::{Deserialize, Serialize};

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

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct IAmAvailable {
    pub chef_addr: Addr<Chef>,
}
