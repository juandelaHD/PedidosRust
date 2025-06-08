use actix::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Message)]
#[serde(tag = "type")]
#[rtype(result = "()")]
pub enum NetworkMessage {
    // Agregar todos los mensajes que se envían a través de la red
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct StartRunning;
