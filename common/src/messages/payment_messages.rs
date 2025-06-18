use crate::types::dtos::OrderDTO;
use actix::Message;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Message sent to request payment authorization for an order.
///
/// # Purpose
/// Used by a coordinator or client to request authorization for a payment from the payment gateway.
///
/// # Contents
/// - `origin_address`: The address of the requester.
/// - `order`: The [`OrderDTO`] representing the order to be authorized.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RequestAuthorization {
    pub origin_address: SocketAddr,
    pub order: OrderDTO,
}

/// Message sent to communicate the result of a payment authorization request.
///
/// # Purpose
/// Used by the payment gateway to inform the requester of the authorization result.
///
/// # Contents
/// - `result`: The [`OrderDTO`] with updated authorization status.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AuthorizationResult {
    pub result: OrderDTO,
}

/// Message sent to notify that payment has been completed for an order.
///
/// # Purpose
/// Used by the payment gateway to inform the requester that payment has been processed.
///
/// # Contents
/// - `order`: The [`OrderDTO`] representing the completed payment.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct PaymentCompleted {
    pub order: OrderDTO,
}

/// Message sent to request billing for a payment.
///
/// # Purpose
/// Used by a coordinator or client to request the payment gateway to bill a payment.
///
/// # Contents
/// - `origin_address`: The address of the requester.
/// - `order`: The [`OrderDTO`] to be billed.
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct BillPayment {
    pub origin_address: SocketAddr,
    pub order: OrderDTO,
}
