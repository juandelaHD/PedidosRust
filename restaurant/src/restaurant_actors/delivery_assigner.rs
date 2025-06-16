use std::{
    collections::{HashMap, VecDeque},
};
use crate::{
    internal_messages::messages::SendThisOrder, restaurant_actors::restaurant::{Restaurant},
};
use actix::{Actor, Handler, Addr};
use common::{
    logger::Logger, messages::{DeliveryAccepted, DeliveryAvailable, RequestNearbyDelivery, UpdateOrderStatus}, types::{dtos::OrderDTO, order_status::OrderStatus, restaurant_info::RestaurantInfo}
};

pub struct DeliveryAssigner {
    /// Información sobre el restaurante.
    pub restaurant_info: RestaurantInfo,
    /// Queue de pedidos listos para ser despachados.
    pub ready_orders: VecDeque<OrderDTO>,
    /// Diccionario de ordenes enviadas y su delivery asignado.
    pub orders_delivery: HashMap<u64, String>,
    /// Comunicador asociado al `Server`.
    pub my_restaurant: Addr<Restaurant>,
    pub logger: Logger,
}

impl DeliveryAssigner {
    pub fn new(
        restaurant_info: RestaurantInfo,
        restaurant_addr: Addr<Restaurant>,
    ) -> Self {
        let logger = Logger::new("DeliveryAssigner");

        DeliveryAssigner {
            restaurant_info,
            ready_orders: VecDeque::new(),
            orders_delivery: HashMap::new(),
            my_restaurant: restaurant_addr,
            logger,
        }
    }
}

use actix::Context;

impl Actor for DeliveryAssigner {
    type Context = Context<Self>;
}

impl Handler<SendThisOrder> for DeliveryAssigner {
    type Result = ();

    fn handle(&mut self, msg: SendThisOrder, _ctx: &mut Self::Context) -> Self::Result {
        // Logic to assign an order to the delivery person
        // For example, you might want to log the order or update some state
        self.logger.info(format!(
            "DeliveryAssigner received order for delivery: {:?}",
            msg.order
        ));
        let mut order = msg.order.clone();
        order.status = OrderStatus::ReadyForDelivery;
        // Add the order to the ready orders queue
        self.ready_orders.push_back(order.clone());
        // Update the order status in the restaurant
        self.my_restaurant.do_send(UpdateOrderStatus {
            order,
        });
        // Notify the server to find nearby deliveries
        self.my_restaurant.do_send(RequestNearbyDelivery {
            order: msg.order.clone(),
            restaurant_info: self.restaurant_info.clone(),
        });
    }
}

impl Handler<DeliveryAvailable> for DeliveryAssigner {
    type Result = ();

    fn handle(&mut self, msg: DeliveryAvailable, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Delivery person available: {}",
            msg.delivery_info.delivery_id
        ));

        // Check if there are ready orders to assign
        if let Some(order) = self.ready_orders.pop_front() {
            // Assign the order to the delivery person
            self.orders_delivery.insert(order.order_id, msg.delivery_info.delivery_id.clone());
            // Notify the restaurant about the assigned order
            self.my_restaurant.do_send(DeliveryAccepted {
                order,
                delivery: msg.delivery_info.clone(),
            });
        } else {
            self.logger.info("No ready orders to assign.");
        }
    }
}