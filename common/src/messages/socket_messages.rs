use actix::prelude::*;

/// Message to stop the socket server
#[derive(Message)]
#[rtype(result = "()")]
pub struct Stop;
