use serde::{Deserialize, Serialize};
use actix::Message;

#[derive(Debug, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub enum PaymentActionType {
    Check,
    Pay,
}