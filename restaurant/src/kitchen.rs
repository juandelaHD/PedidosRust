use actix::prelude::*;
//use common::messages::shared_messages::{NetworkMessage, OrderDTO};
use common::network::communicator::Communicator;
use common::network::peer_types::PeerType;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

//use crate::chef::Chef;
//use crate::mensajes::m::{
//    AssignToChef, IAmAvailable, OrderIsPreparing, SendThisOrder,
//};
//use crate::server_actors::delivery_assigner::DeliveryAssigner;

/// Representa la cocina de un restaurante.
pub struct Kitchen {
    // Ordenes pendientes para ser preparadas.
    //pub pending_orders: VecDeque<OrderDTO>,
    // Chefs disponibles para preparar pedidos.
    //pub chefs_available: Vec<Addr<Chef>>,
    // Comunicador asociado al `Server`.
    //pub communicator: Communicator<Kitchen>,
}