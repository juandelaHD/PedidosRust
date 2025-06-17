use crate::{
    internal_messages::messages::{AssignToChef, IAmAvailable, SendThisOrder},
    restaurant_actors::{delivery_assigner::DeliveryAssigner, kitchen::Kitchen},
};
use actix::{Actor, Addr, AsyncContext, Handler};
use colored::Color;
use common::constants::DEFAULT_TIME_TO_COOK;
use common::{logger::Logger, types::dtos::OrderDTO};
use std::time::Duration;

/// The `Chef` actor is responsible for preparing orders assigned by the kitchen,
/// simulating cooking time, and notifying the delivery assigner and kitchen when appropriate.
///
/// ## Responsibilities:
/// - Receives assigned orders from the kitchen.
/// - Simulates cooking time for each order.
/// - Notifies the delivery assigner when an order is ready.
/// - Notifies the kitchen when available for a new order.
pub struct Chef {
    /// Estimated time to cook an order.
    pub time_to_cook: Duration,
    /// The order currently being prepared.
    pub order: Option<OrderDTO>,
    /// Address of the delivery assigner actor.
    pub delivery_assigner_address: Addr<DeliveryAssigner>,
    /// Address of the kitchen actor.
    pub kitchen_address: Addr<Kitchen>,
    /// Logger for chef events.
    pub logger: Logger,
}

impl Chef {
    /// Creates a new `Chef` actor with the specified delivery assigner and kitchen addresses.
    ///
    /// ## Arguments
    /// * `delivery_assigner_address` - Address of the delivery assigner actor.
    /// * `kitchen_address` - Address of the kitchen actor.
    ///
    /// ## Returns
    /// A new instance of `Chef`.
    pub fn new(
        delivery_assigner_address: Addr<DeliveryAssigner>,
        kitchen_address: Addr<Kitchen>,
    ) -> Self {
        let logger = Logger::new("Chef", Color::BrightBlue);
        Chef {
            delivery_assigner_address,
            kitchen_address,
            time_to_cook: Duration::from_secs(DEFAULT_TIME_TO_COOK),
            order: None,
            logger,
        }
    }
}

impl Actor for Chef {
    type Context = actix::Context<Self>;
}

/// Handles [`AssignToChef`] messages.
///
/// Receives an order assignment from the kitchen, simulates cooking time,
/// and notifies the delivery assigner when the order is ready.
/// After finishing, notifies the kitchen that the chef is available for a new order.
impl Handler<AssignToChef> for Chef {
    type Result = ();

    fn handle(&mut self, msg: AssignToChef, ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Chef received order: {:?}", msg.order.dish_name));
        self.order = Some(msg.order.clone());
        let delivery_assigner = self.delivery_assigner_address.clone();
        let logger = self.logger.clone();
        let kitchen_sender = self.kitchen_address.clone();
        self.logger
            .info(format!("Chef is cooking order: {:?}", msg.order.dish_name));
        ctx.run_later(self.time_to_cook, move |act, ctx| {
            if let Some(order) = &act.order {
                // Notify the delivery assigner that the order is ready
                delivery_assigner.do_send(SendThisOrder {
                    order: order.clone(),
                });
                logger.info(format!("Order {:?} is ready for delivery.", order.order_id));
            } else {
                logger.error("No order assigned to chef when time elapsed.");
            }
            kitchen_sender.do_send(IAmAvailable {
                chef_addr: ctx.address().clone(),
                order: act.order.clone().unwrap(),
            });
        });
    }
}
