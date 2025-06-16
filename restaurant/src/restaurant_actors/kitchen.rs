use actix::prelude::*;
use common::constants::NUMBER_OF_CHEFS;
use common::logger::Logger;
use common::messages::{UpdateOrderStatus};
use common::types::dtos::OrderDTO;
use common::types::order_status::OrderStatus;
//use common::messages::shared_messages::{NetworkMessage, OrderDTO};
use crate::internal_messages::messages::{
    AssignToChef, IAmAvailable, SendToKitchen,
};
use crate::restaurant_actors::chef::Chef;
use crate::restaurant_actors::delivery_assigner::DeliveryAssigner;
use crate::restaurant_actors::restaurant::Restaurant;
use std::collections::VecDeque;

pub struct Kitchen {
    // Ordenes pendientes para ser preparadas.
    pub pending_orders: VecDeque<OrderDTO>,
    // Chefs disponibles para preparar pedidos.
    pub chefs_available: VecDeque<Addr<Chef>>,
    // Comunicador asociado al `Server`.
    pub my_restaurant: Addr<Restaurant>,
    pub my_delivery_assigner: Addr<DeliveryAssigner>,
    pub logger: Logger,
}

impl Kitchen {
    pub fn new(
        my_restaurant: Addr<Restaurant>,
        my_delivery_assigner: Addr<DeliveryAssigner>,
    ) -> Self {
        let logger = Logger::new("Kitchen");
        Kitchen {
            pending_orders: VecDeque::new(),
            chefs_available: VecDeque::new(),
            my_restaurant,
            my_delivery_assigner,
            logger,
        }
    }

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

    fn started(&mut self, ctx: &mut Self::Context) {
        self.logger.info("Kitchen actor started.");
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

impl Handler<SendToKitchen> for Kitchen {
    type Result = ();

    fn handle(&mut self, msg: SendToKitchen, ctx: &mut Self::Context) -> Self::Result {
        self.pending_orders.push_back(msg.order);
        self.assign_orders_to_chefs(ctx);
    }
}

impl Handler<IAmAvailable> for Kitchen {
    type Result = ();

    fn handle(&mut self, msg: IAmAvailable, ctx: &mut Self::Context) -> Self::Result {
        self.chefs_available.push_back(msg.chef_addr);
        self.my_restaurant.do_send(UpdateOrderStatus {
            order: msg.order,
        });
        self.assign_orders_to_chefs(ctx);
    }
}
