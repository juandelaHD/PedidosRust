use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use crate::{
    messages::restaurant_messages::SendThisOrder, restaurant_actors::restaurant::Restaurant,
};
use actix::{Actor, Handler};
use common::{
    logger::Logger,
    messages::{NetworkMessage, RequestDelivery},
    network::communicator::Communicator,
    types::{dtos::OrderDTO, restaurant_info::RestaurantInfo},
};

pub struct DeliveryAssigner {
    /// Información sobre el restaurante.
    pub restaurant_info: RestaurantInfo,
    /// Queue de pedidos listos para ser despachados.
    pub ready_orders: VecDeque<OrderDTO>,
    /// Diccionario de ordenes enviadas y su delivery asignado.
    pub orders_delivery: HashMap<u64, String>,
    /// Comunicador asociado al `Server`.
    pub communicator: Arc<Communicator<Restaurant>>,
    pub logger: Arc<Logger>,
}

impl DeliveryAssigner {
    pub fn new(
        restaurant_info: RestaurantInfo,
        logger: Arc<Logger>,
        communicator: Arc<Communicator<Restaurant>>,
    ) -> Self {
        DeliveryAssigner {
            restaurant_info,
            ready_orders: VecDeque::new(),
            orders_delivery: HashMap::new(),
            communicator,
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
        // Add the order to the ready orders queue
        self.ready_orders.push_back(msg.order.clone());
        // Here you could also implement logic to assign a delivery person
        // For example, you might want to check if a delivery person is available
        if let Some(sender) = self.communicator.sender.as_ref() {
            // Notify the communicator about the order ready for delivery
            sender.do_send(NetworkMessage::RequestDelivery(RequestDelivery {
                order: msg.order.clone(),
                restaurant_info: self.restaurant_info.clone(),
            }));
        } else {
            self.logger.error(
                "Communicator sender is None. Cannot notify order ready for delivery.".to_string(),
            );
        }
        // Store the order in the orders_delivery map with a dummy delivery person name
        self.orders_delivery.insert(
            msg.order.order_id,
            "DeliveryPerson1".to_string(), // This should be replaced with actual delivery person logic
        );
    }
}
