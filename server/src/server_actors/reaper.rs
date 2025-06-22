use crate::messages::internal_messages::{ReapUser, ReconnectUser};
use crate::server_actors::storage::Storage;
use actix::SpawnHandle;
use actix::prelude::*;
use common::constants::REAP_TIMEOUT;
use common::messages::internal_messages::RemoveUser;
use std::collections::HashMap;

/// The `Reaper` actor is responsible for managing user reaping operations.
/// It handles the reaping of users after a specified timeout and manages user reconnections.
///
/// ## Responsibilities
/// - Reaps users after a timeout by sending a message to the `Storage` actor.
/// - Cancels the reaping timer when a user reconnects.
///
/// ## Fields
/// - `users_timer`: A map that associates user IDs with their respective timer handles.
/// - `storage_addr`: The address of the `Storage` actor to which messages are sent
pub struct Reaper {
    /// A map of user IDs to their associated timer handles.
    pub users_timer: HashMap<String, SpawnHandle>,
    /// The address of the storage actor to send messages to.
    pub storage_addr: Addr<Storage>,
}

impl Reaper {
    /// Creates a new `Reaper` actor with an empty user timer map and the specified storage address.
    ///
    /// ## Parameters
    /// - `storage_addr`: The address of the `Storage` actor to send messages to.
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

/// Handles the `ReapUser` messages.
/// This handler sets a timer to reap a user after a specified timeout.
/// When the timer expires, it sends a message to the `Storage` actor to remove the user
/// and removes the user from the timer map.
impl Handler<ReapUser> for Reaper {
    type Result = ();

    fn handle(&mut self, msg: ReapUser, ctx: &mut Self::Context) -> Self::Result {
        let user_id = msg.user_id.clone();
        let handle = ctx.run_later(REAP_TIMEOUT, move |act, _ctx| {
            // Notify the storage actor to reap the user
            act.storage_addr.do_send(RemoveUser {
                user_id: user_id.clone(),
            });
            // Remove the user from the timer map
            act.users_timer.remove(&user_id);
        });
        // Store the handle in the timer map
        self.users_timer.insert(msg.user_id, handle);
    }
}

/// Handles the `ReconnectUser` messages.
/// This handler cancels the timer associated with the user if it exists.
impl Handler<ReconnectUser> for Reaper {
    type Result = ();

    fn handle(&mut self, msg: ReconnectUser, ctx: &mut Self::Context) -> Self::Result {
        if let Some(handle) = self.users_timer.remove(&msg.user_id) {
            ctx.cancel_future(handle);
        }
    }
}
