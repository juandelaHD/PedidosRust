use actix::prelude::*;
use common::messages::AuthorizationResult;
use common::messages::PaymentCompleted;
use common::types::order_status::OrderStatus;

use crate::payment_acceptor::RegisterConnection;
use colored::Color;
use common::logger::Logger;
use common::messages::shared_messages::NetworkMessage;
use common::network::communicator::Communicator;
use common::utils::random_bool_by_given_probability;
use std::collections::HashMap;
use std::collections::HashSet;
use std::net::SocketAddr;

/// The `PaymentGateway` actor simulates a payment gateway that authorizes and charges orders.
///
/// # Responsibilities
/// - Receives authorization and payment requests from coordinators.
/// - Decides whether to authorize an order based on a probability.
/// - Tracks authorized orders and processes payment completion.
/// - Communicates results back to the requesting coordinator via a [`Communicator`].
#[derive(Debug)]
pub struct PaymentGateway {
    /// Set of order IDs that have been authorized for payment.
    pub authorized_orders: HashSet<u64>,
    /// Active communicators mapped by remote address.
    pub communicators: HashMap<SocketAddr, Communicator<PaymentGateway>>,
    /// Probability that an order will be authorized (between 0.0 and 1.0).
    pub probability_of_success: f32,
    /// Logger for payment gateway events.
    pub logger: Logger,
}

impl PaymentGateway {
    /// Creates a new `PaymentGateway` instance.
    ///
    /// # Arguments
    /// * `probability_of_success` - Probability that an order will be authorized.
    pub fn new(probability_of_success: f32) -> Self {
        Self {
            authorized_orders: HashSet::new(),
            communicators: HashMap::new(),
            probability_of_success,
            logger: Logger::new("Payment gateway", Color::BrightWhite),
        }
    }

    /// Sends a network message to the specified destination using the associated communicator.
    ///
    /// # Arguments
    /// * `destination` - The remote address to send the message to.
    /// * `message` - The [`NetworkMessage`] to send.
    pub fn send_network_message(&self, destination: SocketAddr, message: NetworkMessage) {
        if let Some(communicator) = &self.communicators.get(&destination) {
            if let Some(sender) = &communicator.sender {
                sender.do_send(message);
            } else {
                self.logger.error("Sender not initialized in communicator");
            }
        } else {
            self.logger.error("Communicator not found!");
        }
    }
}

impl Actor for PaymentGateway {
    type Context = Context<Self>;
}

/// Handles [`RegisterConnection`] messages to register a new communicator for a remote peer.
impl Handler<RegisterConnection> for PaymentGateway {
    type Result = ();
    fn handle(&mut self, msg: RegisterConnection, _ctx: &mut Self::Context) -> Self::Result {
        self.communicators.insert(msg.client_addr, msg.communicator);
    }
}

/// Handles [`NetworkMessage`] messages for payment authorization and payment completion.
///
/// - On [`RequestAuthorization`], decides to authorize or reject the order.
/// - On [`BillPayment`], completes the payment if the order was previously authorized.
impl Handler<NetworkMessage> for PaymentGateway {
    type Result = ();

    fn handle(&mut self, msg: NetworkMessage, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NetworkMessage::RequestAuthorization(msg) => {
                let mut new_order_dto = msg.order.clone();
                let order_id = new_order_dto.order_id;
                self.logger.info(format!(
                    "New order received: Dish='{}', Client={}, Restaurant={}",
                    new_order_dto.dish_name, new_order_dto.client_id, new_order_dto.restaurant_id
                ));
                let should_authorize =
                    random_bool_by_given_probability(self.probability_of_success);
                if should_authorize {
                    self.logger.info("✅ Order authorized");
                    self.authorized_orders.insert(order_id);
                    new_order_dto.status = OrderStatus::Authorized;
                } else {
                    self.logger.warn("❌ Order rejected");
                    new_order_dto.status = OrderStatus::Unauthorized;
                }
                self.send_network_message(
                    msg.origin_address,
                    NetworkMessage::AuthorizationResult(AuthorizationResult {
                        result: new_order_dto.clone(),
                    }),
                );
            }
            NetworkMessage::BillPayment(msg) => {
                let order_id = msg.order.order_id;

                self.logger
                    .info(format!("💸 Payment successful for order {}", order_id));

                if self.authorized_orders.contains(&order_id) {
                    self.logger.info(format!(
                        "Order {} is authorized, proceeding with payment.",
                        order_id
                    ));
                } else {
                    self.logger.warn(format!(
                        "Order {} is not authorized, cannot proceed with payment.",
                        order_id
                    ));
                    return;
                }
                if let Some(communicator) = self.communicators.get(&msg.origin_address) {
                    if let Some(sender) = &communicator.sender {
                        sender.do_send(NetworkMessage::PaymentCompleted(PaymentCompleted {
                            order: msg.order.clone(),
                        }));
                    } else {
                        self.logger.error("Sender not initialized in communicator");
                    }
                }
            }
            _ => {
                self.logger.error(format!(
                    "Unhandled NetworkMessage in PaymentGateway: {:?}",
                    msg
                ));
            }
        }
    }
}
