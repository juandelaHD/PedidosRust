use actix::Message;
use common::types::dtos::DeliveryDTO;
use serde::{Deserialize, Serialize};

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RecoverProcedure {
    pub delivery_info: DeliveryDTO,
}
