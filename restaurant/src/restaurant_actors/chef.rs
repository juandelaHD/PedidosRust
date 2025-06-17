use crate::{
    internal_messages::messages::{AssignToChef, IAmAvailable, SendThisOrder},
    restaurant_actors::{delivery_assigner::DeliveryAssigner, kitchen::Kitchen},
};
use actix::{Actor, Addr, AsyncContext, Handler};
use common::constants::DEFAULT_TIME_TO_COOK;
use common::{logger::Logger, types::dtos::OrderDTO};
use std::time::Duration;

pub struct Chef {
    /// Tiempo estimado para preparar pedidos.
    pub time_to_cook: Duration,
    /// Pedido que está preparando.
    pub order: Option<OrderDTO>,
    /// Canal de envío hacia el actor `DeliveryAssigner`.
    pub delivery_assigner_address: Addr<DeliveryAssigner>,
    pub kitchen_address: Addr<Kitchen>,
    pub logger: Logger,
}

impl Chef {
    pub fn new(
        delivery_assigner_address: Addr<DeliveryAssigner>,
        kitchen_address: Addr<Kitchen>,
    ) -> Self {
        let logger = Logger::new("Chef");
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

impl Handler<AssignToChef> for Chef {
    type Result = ();

    fn handle(&mut self, msg: AssignToChef, ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Chef received order: {:?}", msg.order.dish_name));
        self.order = Some(msg.order.clone());
        let delivery_assigner = self.delivery_assigner_address.clone();
        let logger = self.logger.clone();
        let kitchen_sender = self.kitchen_address.clone();
        ctx.run_later(self.time_to_cook, move |act, ctx| {
            if let Some(order) = &act.order {
                // Notify the delivery assigner that the order is ready
                delivery_assigner.do_send(SendThisOrder {
                    order: order.clone(),
                });
                logger.info(format!("Order {:?} is ready for delivery.", order.order_id));
            } else {
                logger.error("No order assigned to chef when time elapsed.".to_string());
            }
            kitchen_sender.do_send(IAmAvailable {
                chef_addr: ctx.address().clone(),
                order: act.order.clone().unwrap(),
            });
        });
    }
}
