use actix::prelude::*;
use common::logger::Logger;
use common::messages::{NetworkMessage, OrderIsPreparing};
use common::types::dtos::OrderDTO;
use common::types::order_status::OrderStatus;
//use common::messages::shared_messages::{NetworkMessage, OrderDTO};
use crate::messages::kitchen_messages::{
    AssignToChef, IAmAvailable, SendToKitchen, ShareCommunicator,
};
use crate::restaurant_actors::chef::Chef;
use crate::restaurant_actors::restaurant::Restaurant;
use common::network::communicator::Communicator;
use std::collections::VecDeque;
use std::sync::Arc;

const NUMBER_OF_CHEFS: usize = 4;

pub struct Kitchen {
    // Ordenes pendientes para ser preparadas.
    pub pending_orders: VecDeque<OrderDTO>,
    // Chefs disponibles para preparar pedidos.
    pub chefs_available: VecDeque<Addr<Chef>>,
    // Comunicador asociado al `Server`.
    pub communicator: Arc<Communicator<Restaurant>>,
    pub logger: Arc<Logger>,
}

impl Kitchen {
    pub fn new(
        logger: Arc<Logger>,
        communicator: Arc<Communicator<Restaurant>>,
        chefs: Vec<Addr<Chef>>,
    ) -> Self {
        Kitchen {
            pending_orders: VecDeque::new(),
            chefs_available: VecDeque::from(chefs),
            communicator,
            logger,
        }
    }

    pub fn assign_orders_to_chefs(&mut self, ctx: &mut Context<Kitchen>) {
        while let Some(chef) = self.chefs_available.pop_front() {
            if let Some(mut order) = self.pending_orders.pop_front() {
                order.status = OrderStatus::Preparing;
                if let Some(sender) = self.communicator.sender.as_ref() {
                    sender.do_send(NetworkMessage::OrderIsPreparing(OrderIsPreparing {
                        order: order.clone(),
                    }));
                    chef.do_send(AssignToChef { order });
                    self.logger.info(format!(
                        "Chef {:?} assigned to order {:?}",
                        chef, order.order_id
                    ));
                } else {
                    self.logger.error(
                        "Communicator sender is None. Cannot notify order is preparing."
                            .to_string(),
                    );
                    self.pending_orders.push_front(order);
                    self.chefs_available.push_front(chef);
                    break;
                }
            } else {
                self.chefs_available.push_back(chef);
                break;
            }
        }
    }
}

impl Actor for Kitchen {
    type Context = Context<Self>;
}

impl Handler<ShareCommunicator> for Kitchen {
    type Result = ();

    fn handle(&mut self, _msg: ShareCommunicator, _ctx: &mut Self::Context) -> Self::Result {
        // Ya no se usa Option, así que este handler puede quedar vacío o loguear un warning si se llama
        self.logger.warn(
            "ShareCommunicator handler called, but communicator is already set at construction."
                .to_string(),
        );
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
        self.assign_orders_to_chefs(ctx);
    }
}
