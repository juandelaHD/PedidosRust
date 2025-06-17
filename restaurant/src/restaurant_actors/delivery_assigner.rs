use crate::{
    internal_messages::messages::SendThisOrder, restaurant_actors::restaurant::Restaurant,
};
use actix::{Actor, Addr, Handler};
use common::{
    logger::Logger,
    messages::{DeliverThisOrder, DeliveryAvailable, RequestNearbyDelivery, UpdateOrderStatus},
    types::{dtos::OrderDTO, order_status::OrderStatus, restaurant_info::RestaurantInfo},
};
use std::collections::HashMap;

pub struct DeliveryAssigner {
    /// Información sobre el restaurante.
    pub restaurant_info: RestaurantInfo,
    /// Queue de pedidos listos para ser despachados.
    pub ready_orders: HashMap<u64, OrderDTO>,
    /// Diccionario de ordenes enviadas y su delivery asignado.
    pub orders_delivery: HashMap<u64, String>,
    /// Comunicador asociado al `Server`.
    pub my_restaurant: Addr<Restaurant>,
    pub logger: Logger,
}

impl DeliveryAssigner {
    pub fn new(restaurant_info: RestaurantInfo, restaurant_addr: Addr<Restaurant>) -> Self {
        let logger = Logger::new("DeliveryAssigner");

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

impl Handler<SendThisOrder> for DeliveryAssigner {
    type Result = ();

    fn handle(&mut self, msg: SendThisOrder, _ctx: &mut Self::Context) -> Self::Result {
        // Logic to assign an order to the delivery person
        // For example, you might want to log the order or update some state
        self.logger.info(format!(
            "Received order to send: {:?} for: {:?}",
            msg.order.dish_name, msg.order.client_id
        ));
        let mut order = msg.order.clone();
        order.status = OrderStatus::ReadyForDelivery;
        // Add the order to the ready orders queue
        self.ready_orders.insert(order.order_id, order.clone());
        // Update the order status in the restaurant
        self.my_restaurant.do_send(UpdateOrderStatus { order });
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
            "Delivery '{}' is ready to take an order!",
            msg.delivery_info.delivery_id
        ));
        // chequeo si esa orden esta en ready_orders
        if let Some(order) = self.ready_orders.get(&msg.order.order_id) {
            // Si la orden esta lista, asignamos el delivery
            self.orders_delivery
                .insert(order.order_id, msg.delivery_info.delivery_id.clone());
            // Actualizamos el delivery del pedido
            let mut new_order = order.clone();
            new_order.delivery_id = Some(msg.delivery_info.delivery_id.clone());
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
