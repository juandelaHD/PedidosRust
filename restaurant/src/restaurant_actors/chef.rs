use crate::messages::restaurant_messages::AssignToChef;
use actix::{Actor, Handler};

pub struct Chef;

impl Chef {
    pub fn new() -> Self {
        Chef
    }
}

impl Actor for Chef {
    type Context = actix::Context<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        // Initialization logic for Chef actor can go here
    }
}

impl Handler<AssignToChef> for Chef {
    type Result = ();

    fn handle(&mut self, msg: AssignToChef, _ctx: &mut Self::Context) -> Self::Result {
        // Logic to assign an order to the chef
        // For example, you might want to log the order or update some state
        println!("Chef received order: {:?}", msg.order);
    }
}
