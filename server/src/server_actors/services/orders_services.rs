use crate::messages::internal_messages::{
    AddOrderAccepted, FinishDeliveryAssignment, SetActorsAddresses,
};
use crate::server_actors::coordinator::Coordinator;
use crate::server_actors::storage::Storage;
use actix::prelude::*;
use colored::Color;
use common::logger::Logger;
use common::messages::internal_messages::{
    AddAuthorizedOrderToRestaurant, AddOrder, AddPendingOrderToRestaurant,
    RemoveAuthorizedOrderToRestaurant, RemoveOrder, RemovePendingOrderToRestaurant,
    SetCurrentOrderToDelivery, SetDeliveryToOrder, SetOrderStatus,
};
use common::messages::{
    AcceptedOrder, BillPayment, DeliverThisOrder, DeliveryAccepted, DeliveryAvailable,
    DeliveryNoNeeded, NotifyOrderUpdated, OrderFinalized, RequestAuthorization, RequestThisOrder,
    UpdateOrderStatus,
};
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

/// The `OrderService` actor is responsible for managing orders in the system.
///
/// ## Responsibilities
/// - Receives and processes order requests from clients.
/// - Coordinates payment authorization with the PaymentGateway.
/// - Updates order status and notifies the Coordinator and Storage actors.
/// - Handles delivery assignments and order finalization.
/// - Maintains mappings between clients, restaurants, and their orders.
pub struct OrderService {
    /// Tracks the status of each order by order ID.
    pub orders: HashMap<u64, OrderStatus>,
    /// Maps client IDs to their associated order IDs.
    pub clients_orders: HashMap<String, Vec<u64>>,
    /// Maps restaurant IDs to their associated order IDs.
    pub restaurants_orders: HashMap<String, Vec<u64>>,
    /// List of pending order IDs.
    pub pending_orders: Vec<u64>,
    /// Address of the Coordinator actor.
    pub coordinator_address: Option<Addr<Coordinator>>,
    /// Address of the Storage actor.
    pub storage_address: Option<Addr<Storage>>,
    /// Logger for order service events.
    pub logger: Logger,
    /// Communicator for interacting with the PaymentGateway.
    pub payment_gateway_address: Option<Communicator<OrderService>>,
    /// Pending TCP stream for PaymentGateway connection.
    pub pending_stream: Option<TcpStream>,
}

impl OrderService {
    /// Asynchronously creates a new `OrderService` instance and attempts to connect to the PaymentGateway.
    pub async fn new() -> Self {
        let logger = Logger::new("Order Service", Color::Green);

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

    /// Handles an unauthorized order by notifying the Coordinator.
    ///
    /// ## Arguments
    /// * `order` - The unauthorized [`OrderDTO`].
    /// * `coordinator` - The address of the Coordinator actor.
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

    /// Handles an authorized order by storing it and notifying the Coordinator.
    ///
    /// ## Arguments
    /// * `order` - The authorized [`OrderDTO`].
    /// * `coordinator` - The address of the Coordinator actor.
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

    /// Sends a message to the Storage actor if its address is set.
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

    /// Sends a message to the Coordinator actor if its address is set.
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

    /// Initializes the PaymentGateway communicator when the actor starts.
    fn started(&mut self, ctx: &mut Self::Context) {
        if let Some(stream) = self.pending_stream.take() {
            let communicator = Communicator::new(stream, ctx.address(), PeerType::CoordinatorType);
            self.payment_gateway_address = Some(communicator);
        } else {
            self.logger.error("Failed to connect to PaymentGateway");
        }
    }
}

/// Handles setting the addresses of the Coordinator and Storage actors.
impl Handler<SetActorsAddresses> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: SetActorsAddresses, _ctx: &mut Self::Context) -> Self::Result {
        self.coordinator_address = Some(msg.coordinator_addr);
        self.storage_address = Some(msg.storage_addr);
    }
}

/// Handles order requests from clients by forwarding them to the PaymentGateway for authorization.
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

/// Handles payment authorization results and updates the order accordingly.
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

/// Handles incoming network messages, such as payment completion and authorization results.
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
                // Notificar al  Coordinator para que informe al cliente
                self.send_to_coordinator(NotifyOrderUpdated {
                    peer_id: payment.order.client_id.clone(),
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

/// Handles updates to the status of an order and coordinates changes with Storage and Coordinator.
impl Handler<UpdateOrderStatus> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: UpdateOrderStatus, ctx: &mut Self::Context) -> Self::Result {
        match msg.order.status {
            OrderStatus::Pending => {
                ctx.address().do_send(AddPendingOrderToRestaurant {
                    order: msg.order.clone(),
                    restaurant_id: msg.order.restaurant_id.clone(),
                });
                ctx.address().do_send(SetOrderStatus {
                    order: msg.order.clone(),
                    order_status: OrderStatus::Pending,
                });
            }
            OrderStatus::Cancelled => {
                ctx.address().do_send(RemoveOrder {
                    order: msg.order.clone(),
                });
            }
            OrderStatus::Preparing => {
                ctx.address().do_send(SetOrderStatus {
                    order: msg.order.clone(),
                    order_status: OrderStatus::Preparing,
                });
                ctx.address().do_send(RemovePendingOrderToRestaurant {
                    order: msg.order.clone(),
                    restaurant_id: msg.order.restaurant_id.clone(),
                });
                ctx.address().do_send(AddAuthorizedOrderToRestaurant {
                    order: msg.order.clone(),
                    restaurant_id: msg.order.restaurant_id.clone(),
                });
            }
            OrderStatus::ReadyForDelivery => {
                ctx.address().do_send(SetOrderStatus {
                    order: msg.order.clone(),
                    order_status: OrderStatus::ReadyForDelivery,
                });
            }
            OrderStatus::Delivering => {
                let delivery_id_opt = msg.order.delivery_id.clone();
                if delivery_id_opt.is_none() {
                    self.logger.error(format!(
                        "Order {} is Delivering but has no delivery_id",
                        msg.order.order_id
                    ));
                    return;
                }
                let delivery_id = delivery_id_opt.unwrap();
                ctx.address().do_send(SetCurrentOrderToDelivery {
                    delivery_id: delivery_id.clone(),
                    order: msg.order.clone(),
                });
                ctx.address().do_send(SetDeliveryToOrder {
                    order: msg.order.clone(),
                    delivery_id,
                });
                ctx.address().do_send(SetOrderStatus {
                    order: msg.order.clone(),
                    order_status: OrderStatus::Delivering,
                });
                ctx.address().do_send(RemoveAuthorizedOrderToRestaurant {
                    order: msg.order.clone(),
                    restaurant_id: msg.order.restaurant_id.clone(),
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

/// Handles notifications that a delivery agent has accepted an order.
impl Handler<DeliveryAccepted> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: DeliveryAccepted, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Delivery {} accepted order {}",
            msg.delivery.delivery_id, msg.order.order_id
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

/// Handles notifications that an order has been accepted by a delivery agent and stores the assignment.
impl Handler<AcceptedOrder> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: AcceptedOrder, ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Reenviando AcceptOrder al Storage: {:?}",
            msg.order
        ));
        let message = AddOrderAccepted {
            order: msg.order.clone(),
            delivery: msg.delivery_info.clone(),
            addr: ctx.address().clone(),
        };
        self.send_to_storage(message);
    }
}

/// Handles notifications that a delivery agent is available for an order.
impl Handler<DeliveryAvailable> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: DeliveryAvailable, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Delivery {} is available",
            msg.delivery_info.delivery_id
        ));
        self.send_to_coordinator(msg.clone());
    }
}

/// Handles notifications that a delivery agent is no longer needed for an order.
impl Handler<DeliveryNoNeeded> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: DeliveryNoNeeded, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Delivery {} is not needed for order {}",
            msg.delivery_info.delivery_id, msg.order.order_id
        ));
        // Notificar al Coordinator para que informe al cliente
        self.send_to_coordinator(msg);
    }
}

/// Handles notifications that an order has been finalized (delivered or cancelled).
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

/// Handles instructions to finish a delivery assignment for an order.
impl Handler<DeliverThisOrder> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: DeliverThisOrder, ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Finishing delivery assignment for order {}",
            msg.order.order_id
        ));
        // Notificar al Coordinator para que informe al cliente
        self.send_to_storage(FinishDeliveryAssignment {
            order: msg.order.clone(),
            addr: ctx.address().clone(),
        });
        self.send_to_coordinator(msg.clone());
    }
}

/// Handles requests to remove an order from the system.
impl Handler<RemoveOrder> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: RemoveOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Reenviando Remove Order al Storage: {:?}",
            msg.order
        ));
        self.send_to_storage(msg.clone());
    }
}

/// Handles requests to add an authorized order to a restaurant.
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

/// Handles requests to add a pending order to a restaurant.
impl Handler<AddPendingOrderToRestaurant> for OrderService {
    type Result = ();

    fn handle(
        &mut self,
        msg: AddPendingOrderToRestaurant,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.logger.info(format!(
            "Sending AddPendingOrderToRestaurant to Storage: {:?} -> {}",
            msg.order.order_id.clone(),
            msg.restaurant_id
        ));
        self.send_to_storage(msg.clone());
    }
}

/// Handles requests to remove an authorized order from a restaurant.
impl Handler<RemoveAuthorizedOrderToRestaurant> for OrderService {
    type Result = ();

    fn handle(
        &mut self,
        msg: RemoveAuthorizedOrderToRestaurant,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.logger.info(format!(
            "Sending RemoveAuthorizedOrderToRestaurant to Storage: {:?} -> {}",
            msg.order.order_id.clone(),
            msg.restaurant_id
        ));
        self.send_to_storage(msg);
    }
}

/// Handles requests to remove a pending order from a restaurant.
impl Handler<RemovePendingOrderToRestaurant> for OrderService {
    type Result = ();

    fn handle(
        &mut self,
        msg: RemovePendingOrderToRestaurant,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.logger.info(format!(
            "Sending RemovePendingOrderToRestaurant to Storage: {:?} -> {}",
            msg.order.order_id.clone(),
            msg.restaurant_id
        ));
        self.send_to_storage(msg);
    }
}

/// Handles requests to set the current order for a delivery agent.
impl Handler<SetCurrentOrderToDelivery> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: SetCurrentOrderToDelivery, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Sending SetCurrentOrderToDelivery to Storage: delivery {} -> order {}",
            msg.delivery_id,
            msg.order.order_id.clone()
        ));
        self.send_to_storage(msg.clone());
    }
}

/// Handles requests to update the status of an order.
impl Handler<SetOrderStatus> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: SetOrderStatus, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Sending SetOrderStatus to Storage: order {} -> status {:?}",
            msg.order.order_id.clone(),
            msg.order_status
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

/// Handles requests to assign a delivery agent to an order.
impl Handler<SetDeliveryToOrder> for OrderService {
    type Result = ();

    fn handle(&mut self, msg: SetDeliveryToOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Sending SetDeliveryToOrder to Storage: delivery {} -> order {}",
            msg.delivery_id,
            msg.order.order_id.clone()
        ));

        self.send_to_storage(msg);
    }
}
