use std::{sync::Arc, time::Duration};

use crate::{
    messages::restaurant_messages::{AssignToChef, IAmAvailable, SendThisOrder},
    restaurant_actors::{delivery_assigner::DeliveryAssigner, kitchen::Kitchen},
};
use actix::{Actor, Addr, AsyncContext, Handler};
use common::{logger::Logger, types::dtos::OrderDTO};

const DEFAULT_TIME_TO_COOK: u64 = 5; // Default time in seconds

pub struct Chef {
    /// Tiempo estimado para preparar pedidos.
    pub time_to_cook: Duration,
    /// Pedido que está preparando.
    pub order: Option<OrderDTO>,
    /// Canal de envío hacia el actor `Kitchen`.
    pub kitchen_sender: Option<Addr<Kitchen>>,
    /// Canal de envío hacia el actor `DeliveryAssigner`.
    pub delivery_assigner: Option<Addr<DeliveryAssigner>>,
    pub logger: Arc<Logger>,
    pub communicator: Arc<common::network::communicator::Communicator<Restaurant>>,
}

impl Chef {
    pub fn new(
        logger: Arc<Logger>,
        communicator: Arc<common::network::communicator::Communicator<Restaurant>>,
    ) -> Self {
        Chef {
            time_to_cook: Duration::from_secs(DEFAULT_TIME_TO_COOK),
            order: None,
            kitchen_sender: None,
            delivery_assigner: None,
            logger,
            communicator,
        }
    }

    pub fn set_kitchen_sender(&mut self, kitchen_addr: Addr<Kitchen>) {
        self.kitchen_sender = Some(kitchen_addr);
    }
    pub fn set_delivery_assigner(&mut self, delivery_assigner: Addr<DeliveryAssigner>) {
        self.delivery_assigner = Some(delivery_assigner);
    }
}

impl Actor for Chef {
    type Context = actix::Context<Self>;
}

impl Handler<AssignToChef> for Chef {
    type Result = ();

    fn handle(&mut self, msg: AssignToChef, ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Chef received order: {:?}", msg.order));
        self.order = Some(msg.order.clone());
        let delivery_assigner = self.delivery_assigner.clone();
        let logger = self.logger.clone();
        let kitchen_sender = self.kitchen_sender.clone();
        ctx.run_later(self.time_to_cook, move |act, ctx| {
            if let Some(order) = act.order.take() {
                if let Some(delivery_assigner) = &delivery_assigner {
                    delivery_assigner.do_send(SendThisOrder { order });
                }
                logger.info(format!("Chef finished cooking order: {:?}", order));
            } else {
                logger.info(format!("Chef had no order to cook."));
            }
            if let Some(kitchen_sender) = &kitchen_sender {
                kitchen_sender.do_send(IAmAvailable {
                    chef_addr: ctx.address(),
                });
            }
        });
    }
}
