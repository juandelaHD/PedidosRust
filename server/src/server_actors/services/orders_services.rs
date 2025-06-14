use crate::messages::internal_messages::{
    AddAuthorizedOrderToRestaurant, AddOrder, AddPendingOrderToRestaurant,
    RemoveAuthorizedOrderToRestaurant, RemoveOrder, RemovePendingOrderToRestaurant,
    SetCurrentOrderToDelivery, SetDeliveryToOrder, SetOrderStatus,
};
use crate::server_actors::coordinator::Coordinator;
use crate::server_actors::storage::Storage;
use actix::prelude::*;
use common::logger::Logger;
use common::{
    constants::{PAYMENT_GATEWAY_PORT, SERVER_IP_ADDRESS},
    messages::{AuthorizationResult, NetworkMessage, NewOrder},
    network::{communicator::Communicator, connections::try_to_connect, peer_types::PeerType},
    types::order_status::OrderStatus, 
};
use std::{collections::HashMap, net::SocketAddr};
use tokio::net::TcpStream;

/// OrderService es responsable de:  
/// 1. Reenviar mensajes al Storage.  
/// 2. Notificar al Coordinator para que este informe a los actores externos.
/// 3. Reenviar nuevas órdenes al PaymentGateway.
pub struct OrderService {
    pub orders: HashMap<u64, OrderStatus>,
    pub clients_orders: HashMap<String, Vec<u64>>,
    pub restaurants_orders: HashMap<String, Vec<u64>>,
    pub pending_orders: Vec<u64>,
    pub coordinator_address: Addr<Coordinator>,
    pub storage_address: Addr<Storage>,
    pub logger: Logger,
    pub payment_gateway_address: Option<Communicator<OrderService>>,
    pub pending_stream: Option<TcpStream>,
}

impl OrderService {
    pub async fn new(coordinator_address: Addr<Coordinator>, storage_address: Addr<Storage>) -> Self {
        let logger = Logger::new("OrderService");
        logger.info("Initializing OrderService");

        let payment_gateway_address = format!("{}:{}", SERVER_IP_ADDRESS, PAYMENT_GATEWAY_PORT)
            .parse::<SocketAddr>()
            .expect("Failed to parse server address");

        let pending_stream = try_to_connect(payment_gateway_address).await;
        
        Self {
            orders: HashMap::new(),
            clients_orders: HashMap::new(),
            restaurants_orders: HashMap::new(),
            pending_orders: Vec::new(),
            coordinator_address,
            storage_address,
            logger,
            payment_gateway_address: None,
            pending_stream,
        }
    }
}

impl Actor for OrderService {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.logger.info("OrderService started");
        // Aquí podrías inicializar el PaymentGateway si es necesario
        // self.payment_gateway_address = Some(Communicator::new(PaymentGateway::default()));
        if let Some(stream) = self.pending_stream.take() {
            let communicator = Communicator::new(stream, ctx.address(), PeerType::CoordinatorType);
            self.payment_gateway_address = Some(communicator);
            self.logger.info("Connected to PaymentGateway successfully");
        } else {
            self.logger.error("Failed to connect to PaymentGateway");
        }
    }
}

// Nueva orden REQUESTED al Storage
impl Handler<NewOrder> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: NewOrder, _ctx: &mut Self::Context) -> Self::Result {

        // Notifica al PaymentGateway para que procese el pago
        
    }
}

impl Handler<AuthorizationResult> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: AuthorizationResult, _ctx: &mut Self::Context) -> Self::Result {

        // Si la autorización fue exitosa, se agrega la orden al storage

            // Guardar la orden en la storage
            // Notificar al Coordinator para que informe al Cliente y al Restaurante

        // Si no,   
            // Notificar al Coordinator para que informe al Cliente
        }
    }


impl Handler<NetworkMessage> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: NetworkMessage, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NetworkMessage::AuthorizationResult(result) => {
                // Handle AuthorizationResult
            }
            _ => {
                self.logger.error(format!(
                    "Unhandled NetworkMessage in OrderService: {:?}",
                    msg
                ));
            }
        }
    }
}

impl Handler<RemoveOrder> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: RemoveOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Reenviando RemoveOrder al Storage: {:?}",
            msg.order_id
        ));
        self.storage_address.do_send(msg);
    }

    // Notificar al Coordinator para que informe al Cliente y al Restaurante
}

//
impl Handler<AddAuthorizedOrderToRestaurant> for OrderService {
    type Result = ();

    fn handle(
        &mut self,
        msg: AddAuthorizedOrderToRestaurant,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.logger.info(format!(
            "Reenviando AddAuthorizedOrderToRestaurant al Storage: {:?} -> {}",
            msg.order_id, msg.restaurant_id
        ));
        self.storage_address.do_send(msg);
    }

    // Notificar al Coordinator para que informe al Restaurante
}

impl Handler<AddPendingOrderToRestaurant> for OrderService {
    type Result = ();

    fn handle(
        &mut self,
        msg: AddPendingOrderToRestaurant,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.logger.info(format!(
            "Reenviando AddPendingOrderToRestaurant al Storage: {:?} -> {}",
            msg.order_id, msg.restaurant_id
        ));
        self.storage_address.do_send(msg);
    }
}

impl Handler<RemoveAuthorizedOrderToRestaurant> for OrderService {
    type Result = ();

    fn handle(
        &mut self,
        msg: RemoveAuthorizedOrderToRestaurant,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.logger.info(format!(
            "Reenviando RemoveAuthorizedOrderToRestaurant al Storage: {:?} -> {}",
            msg.order_id, msg.restaurant_id
        ));
        self.storage_address.do_send(msg);
    }
}

impl Handler<RemovePendingOrderToRestaurant> for OrderService {
    type Result = ();

    fn handle(
        &mut self,
        msg: RemovePendingOrderToRestaurant,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.logger.info(format!(
            "Reenviando RemovePendingOrderToRestaurant al Storage: {:?} -> {}",
            msg.order_id, msg.restaurant_id
        ));
        self.storage_address.do_send(msg);
    }
}

impl Handler<SetCurrentOrderToDelivery> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: SetCurrentOrderToDelivery, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Reenviando SetCurrentOrderToDelivery al Storage: delivery {} -> order {}",
            msg.delivery_id, msg.order_id
        ));
        self.storage_address.do_send(msg);
        // Notificar al Coordinator para que informe al Delivery
    }
}

impl Handler<SetOrderStatus> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: SetOrderStatus, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Reenviando SetOrderStatus al Storage: order {} -> status {:?}",
            msg.order_id, msg.order_status
        ));
        self.storage_address.do_send(msg);

        // Obtener la orden del storage
        // Fijarse quien es el cliente, el restaurante y el delivery
        // Mandarle un NotifyStatus al coordinador para cada uno de los tres

        // Notificar al Coordinator para que informe a los actores externos
    }
}

impl Handler<SetDeliveryToOrder> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: SetDeliveryToOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Reenviando SetDeliveryToOrder al Storage: delivery {} -> order {}",
            msg.delivery_id, msg.order_id
        ));

        // 1. Enviar al Storage
        self.storage_address.do_send(SetDeliveryToOrder {
            delivery_id: msg.delivery_id.clone(),
            order_id: msg.order_id,
        });

        // 2. Notificar al Coordinator para que avise al Cliente

    }
}
