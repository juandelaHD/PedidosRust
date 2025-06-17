use crate::internal_messages::messages::{AssignToChef, IAmAvailable, SendToKitchen};
use crate::restaurant_actors::chef::Chef;
use crate::restaurant_actors::delivery_assigner::DeliveryAssigner;
use crate::restaurant_actors::restaurant::Restaurant;
use actix::prelude::*;
use colored::Color;
use common::constants::NUMBER_OF_CHEFS;
use common::logger::Logger;
use common::messages::UpdateOrderStatus;
use common::types::dtos::OrderDTO;
use common::types::order_status::OrderStatus;
use std::collections::VecDeque;

/// The `Kitchen` actor is responsible for managing the queue of orders to be prepared,
/// assigning them to available chefs, and coordinating with the restaurant and delivery assigner.
///
/// ## Responsibilities:
/// - Maintains a queue of pending orders.
/// - Manages available chefs for order preparation.
/// - Assigns orders to chefs as they become available.
/// - Notifies the restaurant and delivery assigner when orders are ready.
pub struct Kitchen {
    /// Queue of orders waiting to be prepared.
    pub pending_orders: VecDeque<OrderDTO>,
    /// Queue of available chefs.
    pub chefs_available: VecDeque<Addr<Chef>>,
    /// Address of the parent restaurant actor.
    pub my_restaurant: Addr<Restaurant>,
    /// Address of the delivery assigner actor.
    pub my_delivery_assigner: Addr<DeliveryAssigner>,
    /// Logger for kitchen events.
    pub logger: Logger,
}

impl Kitchen {
    /// Creates a new `Kitchen` actor with the specified restaurant and delivery assigner addresses.
    ///
    /// Initializes the kitchen with empty queues for pending orders and available chefs.
    ///
    /// ## Arguments
    /// * `my_restaurant` - The address of the restaurant actor.
    /// * `my_delivery_assigner` - The address of the delivery assigner actor.
    pub fn new(
        my_restaurant: Addr<Restaurant>,
        my_delivery_assigner: Addr<DeliveryAssigner>,
    ) -> Self {
        let logger = Logger::new(
            "Kitchen",
            Color::TrueColor {
                r: 255,
                g: 140,
                b: 0,
            },
        );
        Kitchen {
            pending_orders: VecDeque::new(),
            chefs_available: VecDeque::new(),
            my_restaurant,
            my_delivery_assigner,
            logger,
        }
    }

    /// Assigns pending orders to available chefs.
    ///
    /// Iterates through the available chefs and pending orders, assigning each order to a chef
    /// and updating the order status to "Preparing".
    pub fn assign_orders_to_chefs(&mut self, _ctx: &mut Context<Kitchen>) {
        while let Some(chef) = self.chefs_available.pop_front() {
            if let Some(mut order) = self.pending_orders.pop_front() {
                order.status = OrderStatus::Preparing;
                // Notify the restaurant that the order is being prepared
                self.my_restaurant.do_send(UpdateOrderStatus {
                    order: order.clone(),
                });
                // Assign the order to the chef
                chef.do_send(AssignToChef {
                    order: order.clone(),
                });
            } else {
                // If no pending orders, push the chef back to the available queue
                self.chefs_available.push_back(chef);
                break; // Exit the loop if no more orders are pending
            }
        }
    }
}

impl Actor for Kitchen {
    type Context = Context<Self>;

    /// Initializes the kitchen by spawning chefs and assigning any pending orders.
    fn started(&mut self, ctx: &mut Self::Context) {
        // Initialize chefs
        for _ in 0..NUMBER_OF_CHEFS {
            let chef = Chef::new(self.my_delivery_assigner.clone(), ctx.address());
            let chef_addr = chef.start();
            self.chefs_available.push_back(chef_addr);
        }
        // Assign orders to chefs if any are available
        self.assign_orders_to_chefs(ctx);
    }
}

/// Handles [`SendToKitchen`] messages.
///
/// Receives a new order from the restaurant and enqueues it for preparation.
/// Triggers assignment of orders to available chefs.
impl Handler<SendToKitchen> for Kitchen {
    type Result = ();

    fn handle(&mut self, msg: SendToKitchen, ctx: &mut Self::Context) -> Self::Result {
        self.pending_orders.push_back(msg.order);
        self.assign_orders_to_chefs(ctx);
    }
}

/// Handles [`IAmAvailable`] messages.
///
/// Receives notification from a chef that they are available for a new order.
/// Adds the chef to the available queue and attempts to assign pending orders.
impl Handler<IAmAvailable> for Kitchen {
    type Result = ();

    fn handle(&mut self, msg: IAmAvailable, ctx: &mut Self::Context) -> Self::Result {
        self.chefs_available.push_back(msg.chef_addr);
        self.assign_orders_to_chefs(ctx);
    }
}
