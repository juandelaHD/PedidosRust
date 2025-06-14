use actix::prelude::*;
use common::messages::AuthorizationResult;
use common::types::order_status::OrderStatus;

use crate::payment_acceptor::RegisterConnection;
use common::logger::Logger;
use common::messages::shared_messages::NetworkMessage;
use common::network::communicator::Communicator;
use common::utils::random_bool_by_given_probability;
use std::collections::HashMap;
use std::collections::HashSet;
use std::net::SocketAddr;

#[derive(Debug)]
pub struct PaymentGateway {
    pub authorized_orders: HashSet<u64>,
    pub communicators: HashMap<SocketAddr, Communicator<PaymentGateway>>,
    pub probability_of_success: f32,
    pub logger: Logger,
}

impl PaymentGateway {
    pub fn new(probability_of_success: f32) -> Self {
        Self {
            authorized_orders: HashSet::new(),
            communicators: HashMap::new(),
            probability_of_success,
            logger: Logger::new("Payment GATEWAY"),
        }
    }
    pub fn send_network_message(&self, destination: SocketAddr, message: NetworkMessage) {
        if let Some(communicator) = &self.communicators.get(&destination) {
            if let Some(sender) = &communicator.sender {
                sender.do_send(message);
            } else {
                self.logger.error("Sender not initialized in communicator");
            }
        } else {
            self.logger.error(&format!("Communicator not found!",));
        }
    }
}

impl Actor for PaymentGateway {
    type Context = Context<Self>;
}

impl Handler<RegisterConnection> for PaymentGateway {
    type Result = ();
    fn handle(&mut self, msg: RegisterConnection, _ctx: &mut Self::Context) -> Self::Result {
        self.communicators.insert(msg.client_addr, msg.communicator);
    }
}

impl Handler<NetworkMessage> for PaymentGateway {
    type Result = ();

    fn handle(&mut self, msg: NetworkMessage, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NetworkMessage::RequestAuthorization(msg) => {
                let mut new_order_dto = msg.order.clone();
                let order_id = new_order_dto.order_id;
                self.logger
                    .info(format!("New order received: {:?}", new_order_dto));
                // Simulate authorization logic
                let should_authorize =
                    random_bool_by_given_probability(self.probability_of_success);
                if should_authorize {
                    self.logger
                        .info(format!("✅ Order {} authorized", order_id));

                    // Store the authorized order
                    self.authorized_orders.insert(order_id);
                    new_order_dto.status = OrderStatus::Authorized;
                } else {
                    self.logger.warn(format!("❌ Order {} rejected", order_id));
                    new_order_dto.status = OrderStatus::Unauthorized;
                }
                self.send_network_message(
                    msg.origin_address,
                    NetworkMessage::AuthorizationResult(AuthorizationResult {
                        result: new_order_dto.clone(),
                    }),
                );
            }
            _ => {
                // Handle other message types as needed
                self.logger.error(format!(
                    "Unhandled NetworkMessage in PaymentGateway: {:?}",
                    msg
                ));
            }
        }
    }
}
