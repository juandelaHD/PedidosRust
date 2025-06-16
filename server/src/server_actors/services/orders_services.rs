use crate::messages::internal_messages::{
    AddAuthorizedOrderToRestaurant, AddOrder, AddPendingOrderToRestaurant, RemoveAuthorizedOrderToRestaurant, RemoveOrder, RemovePendingOrderToRestaurant, SetActorsAddresses, SetCurrentOrderToDelivery, SetDeliveryToOrder, SetOrderStatus
};
use crate::server_actors::coordinator::Coordinator;
use crate::server_actors::storage::Storage;
use actix::prelude::*;
use common::logger::Logger;
use common::messages::{BillPayment, DeliveryAccepted, NotifyOrderUpdated, OrderFinalized, RequestAuthorization, RequestThisOrder, UpdateOrderStatus};
use common::network::connections::connect_some;
use common::types::dtos::OrderDTO;
use common::{
    constants::{PAYMENT_GATEWAY_PORT, SERVER_IP_ADDRESS},
    messages::{AuthorizationResult, NetworkMessage, NewOrder},
    network::{communicator::Communicator, peer_types::PeerType},
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
    pub coordinator_address: Option<Addr<Coordinator>>,
    pub storage_address: Option<Addr<Storage>>,
    pub logger: Logger,
    pub payment_gateway_address: Option<Communicator<OrderService>>,
    pub pending_stream: Option<TcpStream>,
}

impl OrderService {
    pub async fn new() -> Self {
        let logger = Logger::new("OrderService");
        logger.info("Initializing OrderService");

        let payment_gateway_address = format!("{}:{}", SERVER_IP_ADDRESS, PAYMENT_GATEWAY_PORT)
            .parse::<SocketAddr>()
            .expect("Failed to parse server address");

        let pending_stream =
            connect_some(vec![payment_gateway_address], PeerType::CoordinatorType).await;

        Self {
            orders: HashMap::new(),
            clients_orders: HashMap::new(),
            restaurants_orders: HashMap::new(),
            pending_orders: Vec::new(),
            coordinator_address: None,
            storage_address: None,
            logger,
            payment_gateway_address: None,
            pending_stream,
        }
    }

    fn handle_unauthorized_order(&mut self, order: &OrderDTO, coordinator: Addr<Coordinator>) {
        self.logger.warn(format!(
            "Order {} unauthorized, notifying Coordinator",
            order.order_id
        ));
        coordinator.do_send(NotifyOrderUpdated {
            peer_id: order.client_id.clone(),
            order: order.clone(),
        });
    }

    fn handle_authorized_order(&mut self, order: &OrderDTO, coordinator: Addr<Coordinator>) {
        self.logger.info(format!(
            "Order {} authorized, notifying Coordinator",
            order.order_id
        ));
        if let Some(addr) = self.storage_address.as_ref() {
            addr.do_send(AddOrder {
                order: order.clone(),
            });
        } else {
            self.logger.error("Storage address not set");
        }
        coordinator.do_send(NewOrder {
            order: order.clone(),
        });
        coordinator.do_send(NotifyOrderUpdated {
            peer_id: order.client_id.clone(),
            order: order.clone(),
        });
    }
    
    fn send_to_storage<T>(&self, msg: T)
        where
            T: Message + Send + 'static,
            T::Result: Send + 'static,
            Storage: Handler<T>,
            <Storage as Handler<T>>::Result: Send,
        {
        if let Some(addr) = self.storage_address.as_ref() {
                addr.do_send(msg);
            } else {
                self.logger.error("Storage address not set");
            }
        }

        fn send_to_coordinator<T>(&self, msg: T)
        where
            T: Message + Send + 'static,
            T::Result: Send + 'static,
            Coordinator: Handler<T>,
            <Coordinator as Handler<T>>::Result: Send,
        {
        if let Some(addr) = self.coordinator_address.as_ref() {
                addr.do_send(msg);
            } else {
                self.logger.error("Storage address not set");
            }
        }

}

impl Actor for OrderService {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.logger.info("OrderService started");
        if let Some(stream) = self.pending_stream.take() {
            let communicator = Communicator::new(stream, ctx.address(), PeerType::CoordinatorType);
            self.payment_gateway_address = Some(communicator);
            self.logger.info("Connected to PaymentGateway successfully");
        } else {
            self.logger.error("Failed to connect to PaymentGateway");
        }
    }
}

impl Handler<SetActorsAddresses> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: SetActorsAddresses, _ctx: &mut Self::Context) -> Self::Result {
        self.coordinator_address = Some(msg.coordinator_addr);
        self.storage_address = Some(msg.storage_addr);
    }
}

impl Handler<RequestThisOrder> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: RequestThisOrder, _ctx: &mut Self::Context) -> Self::Result {
        // Notifica al PaymentGateway para que procese el pago
        if let Some(communicator) = self.payment_gateway_address.as_ref() {
            if let Some(sender) = communicator.sender.as_ref() {
                let socket_addr = communicator.local_address;
                let auth_message = NetworkMessage::RequestAuthorization(RequestAuthorization {
                    origin_address: socket_addr,
                    order: msg.order,
                });
                sender.do_send(auth_message);
            } else {
                self.logger
                    .error("PaymentGateway Communicator sender not initialized");
            }
        } else {
            self.logger
                .error("PaymentGateway Communicator not initialized");
        }
    }
}

impl Handler<AuthorizationResult> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: AuthorizationResult, _ctx: &mut Self::Context) -> Self::Result {
        let order = msg.result;
        if let Some(coordinator) = &self.coordinator_address {
            match order.status {
                OrderStatus::Authorized => {
                    self.handle_authorized_order(&order, coordinator.clone());
                }
                OrderStatus::Unauthorized => {
                    self.handle_unauthorized_order(&order, coordinator.clone());
                }
                _ => {
                    self.logger.error(format!(
                        "Unexpected order status {:?} for order {}",
                        order.status, order.order_id
                    ));
                }
            }
        } else {
            self.logger.error("Coordinator address not set");
        }
    }
}

impl Handler<NetworkMessage> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: NetworkMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NetworkMessage::AuthorizationResult(result) => {
                ctx.address().do_send(result);
            }
            NetworkMessage::PaymentCompleted(payment) => {
                self.logger.info(format!(
                    "Payment completed for order {}: {:?}",
                    payment.order.order_id, payment
                ));
                // Como se terminó la entrega, se elimina la orden del Storage
                self.send_to_storage(RemoveOrder {
                    order: payment.order.clone(),
                });
                // Notificar al  Coordinator para que informe al restaurante
                self.send_to_coordinator(OrderFinalized {
                    order: payment.order.clone(),
                });
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

impl Handler<UpdateOrderStatus> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: UpdateOrderStatus, ctx: &mut Self::Context) -> Self::Result {
        
        match msg.order.status {
            OrderStatus::Pending => {
                ctx.address().do_send(AddPendingOrderToRestaurant {
                    order: msg.order.clone(),
                    restaurant_id: msg.order.restaurant_id.clone(),
                });
            }
            OrderStatus::Cancelled => {
                ctx.address().do_send(RemoveOrder {
                    order: msg.order.clone(),
                });
            }
            OrderStatus::Preparing => {
                ctx.address().do_send( SetOrderStatus{
                    order: msg.order.clone(),
                    order_status: OrderStatus::Preparing,
                });
            }
            OrderStatus::ReadyForDelivery => {
                ctx.address().do_send(SetOrderStatus {
                    order: msg.order.clone(),
                    order_status: OrderStatus::ReadyForDelivery,
                });
            }
            OrderStatus::Delivering => {
                let delivery_id = msg.order.delivery_id.clone();
                if delivery_id.is_none() {
                    self.logger.error(format!(
                        "Order {} is Delivering but has no delivery_id",
                        msg.order.order_id
                    ));
                    return;
                }
                ctx.address().do_send(SetCurrentOrderToDelivery {
                    delivery_id: delivery_id.unwrap(),
                    order: msg.order.clone(),
                });
            }
            OrderStatus::Delivered => {
                ctx.address().do_send(OrderFinalized {
                    order: msg.order.clone(),
                });
            }
            _ => {
                self.logger.error(format!(
                    "Unexpected order status {:?} for order {}",
                    msg.order.status, msg.order.order_id
                ));
            }
        }
    }
}

impl Handler<DeliveryAccepted> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: DeliveryAccepted, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Delivery accepted for order {} by delivery {}",
            msg.order.order_id, msg.delivery.delivery_id
        ));
        // Notificar al Coordinator para que informe al cliente
        if let Some(coordinator) = &self.coordinator_address {
            coordinator.do_send(NotifyOrderUpdated {
                peer_id: msg.order.client_id.clone(),
                order: msg.order.clone(),
            });
        } else {
            self.logger.error("Coordinator address not set");
        }
    }
}

impl Handler<OrderFinalized> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: OrderFinalized, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Reenviando CancelOrder al Storage: {:?}",
            msg.order
        ));
        if let Some(communicator) = self.payment_gateway_address.as_ref() {
            let socket_addr = communicator.local_address;
            if let Some(sender) = communicator.sender.as_ref() {
                let auth_message = NetworkMessage::BillPayment(BillPayment {
                    origin_address: socket_addr,
                    order: msg.order.clone(),
                });
                sender.do_send(auth_message);
            } else {
                self.logger
                    .error("PaymentGateway Communicator sender not initialized");
            }
        } else {
            self.logger
                .error("PaymentGateway Communicator not initialized");
        }
    }
}

impl Handler<RemoveOrder> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: RemoveOrder, _ctx: &mut Self::Context) -> Self::Result {
            self.logger.info(format!(
                "Reenviando RemoveOrder al Storage: {:?}",
                msg.order
            ));
            self.send_to_storage(msg.clone());
            // Notificar al Coordinator para que informe al cliente
            self.send_to_coordinator( NotifyOrderUpdated {
                peer_id: msg.order.client_id.clone(),
                order: msg.order.clone(),
            });
        }
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
            "Sending AddAuthorizedOrderToRestaurant to Storage: {:?} -> {}",
            msg.order, msg.restaurant_id
        ));
        self.send_to_storage(msg);
    }
}

impl Handler<AddPendingOrderToRestaurant> for OrderService {
    type Result = ();

    fn handle(
        &mut self,
        msg: AddPendingOrderToRestaurant,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.logger.info(format!(
            "Sending AddPendingOrderToRestaurant to Storage: {:?} -> {}",
            msg.order.order_id.clone(), msg.restaurant_id
        ));
        self.send_to_storage(msg.clone());
        // Notificar al Coordinator para que informe al cliente
        self.send_to_coordinator(NotifyOrderUpdated {
            peer_id: msg.order.client_id.to_string(),
            order: msg.order.clone(), 
        });
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
            "Sending RemoveAuthorizedOrderToRestaurant to Storage: {:?} -> {}",
            msg.order.order_id.clone(), msg.restaurant_id
        ));
        self.send_to_storage(msg);
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
            "Sending RemovePendingOrderToRestaurant to Storage: {:?} -> {}",
            msg.order.order_id.clone(), msg.restaurant_id
        ));
        self.send_to_storage(msg);
    }
}

impl Handler<SetCurrentOrderToDelivery> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: SetCurrentOrderToDelivery, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Sending SetCurrentOrderToDelivery to Storage: delivery {} -> order {}",
            msg.delivery_id, msg.order.order_id.clone()
        ));
        self.send_to_storage(msg);
        // Notificar al Coordinator para que informe al Delivery
    }
}

impl Handler<SetOrderStatus> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: SetOrderStatus, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Sending SetOrderStatus to Storage: order {} -> status {:?}",
            msg.order.order_id.clone(), msg.order_status
        ));
        if let Some(addr) = self.storage_address.as_ref() {
            addr.do_send(msg.clone());
        } else {
            self.logger.error("Storage address not set");
        }
        // Notificar al Coordinator para que informe al cliente
        self.send_to_coordinator(NotifyOrderUpdated {
            peer_id: msg.order.client_id.clone(),
            order: msg.order.clone(),
        });
    }
}

impl Handler<SetDeliveryToOrder> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: SetDeliveryToOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Sending SetDeliveryToOrder to Storage: delivery {} -> order {}",
            msg.delivery_id, msg.order.order_id.clone()
        ));

        self.send_to_storage(msg);

        // 2. Notificar al Coordinator para que avise al Cliente
        
    }
}
