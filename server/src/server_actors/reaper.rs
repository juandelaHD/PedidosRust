use std::collections::HashMap;
use actix::prelude::*;
use actix::SpawnHandle;
use crate::server_actors::storage::Storage;
use crate::messages::internal_messages::{ReapUser, ReconnectUser};
use common::messages::internal_messages::RemoveUser;
use common::constants::REAP_TIMEOUT;

pub struct Reaper {

    pub users_timer: HashMap<String, SpawnHandle>,
    pub storage_addr: Addr<Storage>,
}


impl Reaper {
    pub fn new(storage_addr: Addr<Storage>) -> Self {
        Reaper {
            users_timer: HashMap::new(),
            storage_addr,
        }
    }


}

impl Actor for Reaper {
    type Context = Context<Self>;
}




impl Handler<ReapUser> for Reaper {
    type Result = ();

    fn handle(&mut self, msg: ReapUser, ctx: &mut Self::Context) -> Self::Result {
        let user_id = msg.user_id.clone();
        let handle = ctx.run_later(REAP_TIMEOUT, move |act, _ctx| {
            // Notify the storage actor to reap the user
            act.storage_addr.do_send(RemoveUser { user_id: user_id.clone() });
            // Remove the user from the timer map
            act.users_timer.remove(&user_id);
        });
        // Store the handle in the timer map
        self.users_timer.insert(msg.user_id, handle);

    }
}


impl Handler<ReconnectUser> for Reaper {
    type Result = ();

    fn handle(&mut self, msg: ReconnectUser, ctx: &mut Self::Context) -> Self::Result {
        if let Some(handle) = self.users_timer.remove(&msg.user_id) {
            ctx.cancel_future(handle);
        }
        
    }
}