use actix::prelude::*;
use common::logger::Logger;
use common::messages::shared_messages::NetworkMessage;
use common::network::communicator::Communicator;
use serde::Serialize;
use std::sync::Arc;
use std::{collections::HashMap, net::SocketAddr};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::mensajes::m::RegisterConnection;

#[derive(Debug)]
pub struct Coordinator {
    /// Dirección de este coordinator.
    pub addr: SocketAddr,
    /// Coordinador actual.
    pub current_coordinator: Option<SocketAddr>,
    /// Estado de los pedidos en curso.
    //   pub active_orders: HashSet<u64>, // TODO: Ver si se puede sacar
    /// Comunicador con el PaymentGateway
    // pub payment_communicator: Communicator,
    /// Diccionario de conexiones activas con clientes, restaurantes y deliverys.
    pub communicators: HashMap<SocketAddr, Communicator<Coordinator>>,
    /// Diccionario de direcciones de usuarios con sus correspondientes IDs.
    pub user_addresses: HashMap<SocketAddr, String>,
    /// Canal de envío hacia el actor `Storage`.
    // pub storage: Addr<Storage>,
    /// Canal de envío hacia el actor `Reaper`.
    // pub reaper: Addr<Reaper>,
    /// Servicio de órdenes.
    // pub order_service: Addr<OrderService>,
    /// Servicio de restaurantes cercanos.
    // pub nearby_restaurant_service: Addr<NearbyRestaurantService>,
    // Servicio de deliverys cercanos.
    // pub nearby_delivery_service: Addr<NearbyDeliveryService>,
    pub logger: Arc<Logger>,
}

impl Actor for Coordinator {
    type Context = Context<Self>;
}

impl Coordinator {
    pub fn new(srv_addr: SocketAddr) -> Addr<Self> {
        Coordinator::create(move |ctx| Coordinator {
            addr: srv_addr,
            current_coordinator: None,
            communicators: HashMap::new(),
            user_addresses: HashMap::new(),
            logger: Arc::new(Logger::new(format!("Coordinator {}", srv_addr))),
        })
    }
}

impl Handler<NetworkMessage> for Coordinator {
    type Result = ();
    fn handle(&mut self, _msg: NetworkMessage, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info("Received message\n");
    }
}

impl Handler<RegisterConnection> for Coordinator {
    type Result = ();
    fn handle(&mut self, msg: RegisterConnection, _ctx: &mut Self::Context) -> Self::Result {
        // Registrar la conexión del cliente
        self.communicators.insert(msg.client_addr, msg.communicator);

        // TODO: El valor debe ser el ID del cliente (el nombre)
        self.user_addresses
            .insert(msg.client_addr, msg.client_addr.to_string());
        self.logger
            .info(format!("Registered connection from {} ", msg.client_addr));
        // Aquí podrías enviar un mensaje de bienvenida o iniciar alguna lógica adicional
    }
}
