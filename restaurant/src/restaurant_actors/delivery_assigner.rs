use crate::{
    internal_messages::messages::SendThisOrder, restaurant_actors::restaurant::Restaurant,
};
use actix::{Actor, Addr, Handler};
use colored::Color;
use common::{
    logger::Logger,
    messages::{
        CancelOrder, DeliverThisOrder, DeliveryAvailable, RequestNearbyDelivery, UpdateOrderStatus,
    },
    types::{dtos::OrderDTO, order_status::OrderStatus, restaurant_info::RestaurantInfo},
};
use std::collections::HashMap;

/// The `DeliveryAssigner` actor is responsible for tracking orders that are ready for delivery,
/// assigning them to available delivery personnel, and notifying the restaurant and delivery actors.
/// ## Responsibilities:
/// - Tracks orders that are ready for delivery.
/// - Matches available deliveries to ready orders.
/// - Notifies the restaurant and delivery actors when an order is assigned.
pub struct DeliveryAssigner {
    /// Information about the restaurant.
    pub restaurant_info: RestaurantInfo,
    /// Orders ready to be dispatched.
    pub ready_orders: HashMap<u64, OrderDTO>,
    /// Mapping of orders to assigned delivery IDs.
    pub orders_delivery: HashMap<u64, String>,
    /// Address of the parent restaurant actor.
    pub my_restaurant: Addr<Restaurant>,
    /// Logger for delivery assigner events.
    pub logger: Logger,
}

impl DeliveryAssigner {
    /// Creates a new `DeliveryAssigner` actor.
    ///
    /// # Arguments
    /// * `restaurant_info` - Information about the restaurant.
    /// * `restaurant_addr` - Address of the restaurant actor.
    ///
    /// # Returns
    /// A new instance of `DeliveryAssigner`.
    pub fn new(restaurant_info: RestaurantInfo, restaurant_addr: Addr<Restaurant>) -> Self {
        let logger = Logger::new("Delivery Assigner", Color::BrightWhite);

        DeliveryAssigner {
            restaurant_info,
            ready_orders: HashMap::new(),
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

/// Handles [`SendThisOrder`] messages.
///
/// Receives notification from the kitchen that an order is ready for delivery.
/// Adds the order to the ready queue, updates its status, and requests nearby deliveries from the server.
impl Handler<SendThisOrder> for DeliveryAssigner {
    type Result = ();

    fn handle(&mut self, msg: SendThisOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Received order to send: {:?} for: {:?}",
            msg.order.dish_name, msg.order.client_id
        ));
        let mut order = msg.order.clone();
        order.status = OrderStatus::ReadyForDelivery;
        self.ready_orders.insert(order.order_id, order.clone());
        self.my_restaurant.do_send(UpdateOrderStatus {
            order: order.clone(),
        });
        self.my_restaurant.do_send(RequestNearbyDelivery {
            order: order.clone(),
            restaurant_info: self.restaurant_info.clone(),
        });
    }
}

/// Handles [`DeliveryAvailable`] messages.
///
/// Receives notification from the server that a delivery person is available.
/// Assigns the ready order to the delivery person and notifies the restaurant to proceed with delivery.
impl Handler<DeliveryAvailable> for DeliveryAssigner {
    type Result = ();

    fn handle(&mut self, msg: DeliveryAvailable, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Delivery '{}' is ready to take an order!",
            msg.delivery_info.delivery_id
        ));
        // chequeo si esa orden esta en ready_orders
        if let Some(order) = self.ready_orders.get(&msg.order.order_id) {
            // chequeo si el delivery no es none:
            if msg.delivery_info.delivery_id.is_empty() {
                self.logger
                    .warn("Delivery ID is empty, cannot assign order.");
                // TODO: CANCELAR LA ORDEN;
            }
            // Si la orden esta lista, asignamos el delivery
            self.orders_delivery
                .insert(order.order_id, msg.delivery_info.delivery_id.clone());
            // Actualizamos el delivery del pedido
            let mut new_order = order.clone();
            new_order.delivery_id = Some(msg.delivery_info.delivery_id.clone());
            new_order.status = OrderStatus::Delivering;
            // Avisamos al delivery que puede buscar la orden y enviarla
            self.my_restaurant.do_send(DeliverThisOrder {
                order: new_order,
                restaurant_info: self.restaurant_info.clone(),
            });
        } else {
            self.logger.warn(format!(
                "No ready order found for {}",
                msg.delivery_info.delivery_id
            ));
        }
    }
}

/// Handles [`CancelOrder`] messages.
///
/// Receives a cancellation request for an order and removes it from the ready queue.
impl Handler<CancelOrder> for DeliveryAssigner {
    type Result = ();

    fn handle(&mut self, msg: CancelOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .warn(format!("Cancelling order: {}", msg.order.order_id));
        // Remove the order from ready orders if it exists
        self.ready_orders.remove(&msg.order.order_id);
    }
}

impl Drop for DeliveryAssigner {
    fn drop(&mut self) {
        self.logger
            .info("AAAAAAAAAAAAAAAa Delivery Assigner is being dropped.");
    }
}
