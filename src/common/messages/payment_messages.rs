use actix::prelude::*;
use serde::{Deserialize, Serialize};
use crate::common::types::payment_status::PaymentActionType;
use std::net::SocketAddr;

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub enum PaymentGatewayRequest {
    CheckAuth(RequestPaymentAuthorization),
    Execute(RequestPaymentExecution),
}

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct RequestPaymentAuthorization {
    pub client_id: String,
    pub amount: f32,
}

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct PaymentAuthorizationResult {
    pub client_id: String,
    pub authorized: bool,
}

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct RequestPaymentExecution {
    pub client_id: String,
    pub amount: f32,
}

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub enum PaymentGatewayResponse {
    Success,
    Error(String),
}

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "(bool)")]
pub struct GatewaySendPaymentMessage {
    pub client_id: String,
    pub amount: f32,
    pub message_type: PaymentActionType,
}

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct GatewayAuthConfirmation {
    pub client_id_addr: SocketAddr,
    pub is_authorized: bool,
}