use crate::messages::internal_messages::{
    AddOrderAccepted, FinishDeliveryAssignment, GetAllStorage, GetLogsFromIndex, GetMinLogIndex,
};
use crate::server_actors::coordinator::Coordinator;
use actix::prelude::*;
use colored::Color;
use common::bimap::BiMap;
use common::logger::Logger;
use common::messages::coordinatormanager_messages::StorageSnapshot;
use common::messages::internal_messages::{
    AddAuthorizedOrderToRestaurant, AddClient, AddDelivery, AddOrder, AddPendingOrderToRestaurant,
    AddRestaurant, ApplyStorageUpdates, GetAllAvailableDeliveries, GetAllRestaurantsInfo,
    GetClient, GetDeliveries, GetDelivery, GetOrder, RemoveUser, GetRestaurant, GetRestaurants,
    InsertAcceptedDelivery, RemoveAcceptedDeliveries, RemoveAuthorizedOrderToRestaurant,
    RemoveClient, RemoveDelivery, RemoveOrder, RemovePendingOrderToRestaurant, RemoveRestaurant,
    SetCurrentClientToDelivery, SetCurrentOrderToDelivery, SetDeliveryPosition, SetDeliveryStatus,
    SetDeliveryToOrder, SetOrderExpectedTime, SetOrderStatus, StorageLogMessage,
};
use common::messages::{DeliveryAvailable, DeliveryNoNeeded};
use common::types::order_status::OrderStatus;
use common::types::{
    dtos::{ClientDTO, DeliveryDTO, OrderDTO, RestaurantDTO, Snapshot},
    restaurant_info::RestaurantInfo,
};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

/// The `Storage` actor is responsible for maintaining and updating all persistent state in the system,
/// including clients, restaurants, deliveries, orders, and the storage log.
///
/// # Responsibilities
/// - Stores and manages all entities (clients, restaurants, deliveries, orders).
/// - Handles log-based persistence and synchronization for distributed recovery.
/// - Applies and logs all state-changing operations.
/// - Provides snapshots and log segments for recovery and replication.
/// - Coordinates with the `Coordinator` actor for system-wide updates.
pub struct Storage {
    /// Dictionary with information about clients.
    pub clients: HashMap<String, ClientDTO>,
    /// Dictionary with information about restaurants.
    pub restaurants: HashMap<String, RestaurantDTO>,
    /// Dictionary with information about deliveries.
    pub deliverys: HashMap<String, DeliveryDTO>,
    /// Dictionary of orders.
    pub orders: HashMap<u64, OrderDTO>,
    /// Deliveries that have accepted orders.
    pub accepted_deliveries: BiMap<u64, String>,
    /// List of storage log updates.
    pub storage_updates: HashMap<u64, StorageLogMessage>,
    /// Index of the next log entry.
    pub next_log_id: u64,
    /// Index of the minimum persistent operation in the log.
    pub min_persistent_log_index: u64,
    /// Address of the associated `Coordinator`.
    pub coordinator: Addr<Coordinator>,
    /// Logger for storage events.
    pub logger: Logger,
}

impl Storage {
    /// Creates a new `Storage` actor instance.
    ///
    /// # Arguments
    /// * `coordinator` - The address of the `Coordinator` actor.
    pub fn new(coordinator: Addr<Coordinator>) -> Self {
        Self {
            clients: HashMap::new(),
            restaurants: HashMap::new(),
            deliverys: HashMap::new(),
            orders: HashMap::new(),
            accepted_deliveries: BiMap::new(),
            storage_updates: HashMap::new(),
            next_log_id: 1,
            min_persistent_log_index: 0,
            coordinator,
            logger: Logger::new("Storage", Color::White),
        }
    }
    /// Adds a new log entry to the storage log and increments the log index.
    ///
    /// # Arguments
    /// * `log_message` - The [`StorageLogMessage`] to add.
    fn add_to_log(&mut self, log_message: StorageLogMessage) {
        self.storage_updates.insert(self.next_log_id, log_message);
        self.next_log_id += 1;
    }

    fn update_associated_order(&mut self, order: &OrderDTO) {
        // chequemos si la orden existe en el storage
        if let Some(order) = self.orders.get_mut(&order.order_id) {
            // Actualizamos la orden en el storage
            *order = order.clone();
            let order_clone = order.clone();
            // nos fijamos si el cliente existe
            // y actualizamos la orden asociada al cliente
            if let Some(client) = self.clients.get_mut(&order.client_id) {
                client.client_order = Some(order.clone());
            } else {
                self.logger
                    .error(format!("Client not found for order: {}", order.client_id));
            }
            // nos fijamos si el delivery existe
            // y actualizamos la orden asociada al delivery
            if let Some(delivery_id) = &order.delivery_id {
                if let Some(delivery) = self.deliverys.get_mut(delivery_id) {
                    delivery.current_order = Some(order.clone());
                } else {
                    self.logger
                        .error(format!("Delivery not found for order: {}", order.order_id));
                }
            }
            // nos fijamos si el restaurant existe
            if let Some(restaurant) = self.restaurants.get_mut(&order.restaurant_id) {
                // nos fijamos si la orden está en authorized_orders o en pending_orders
                if restaurant.authorized_orders.remove(order) {
                    restaurant.authorized_orders.insert(order_clone);
                } else if restaurant.pending_orders.remove(order) {
                    restaurant.pending_orders.insert(order_clone);
                } else {
                    self.logger.error(format!(
                        "Order not found in restaurant orders: {}",
                        order.order_id
                    ));
                }
            } else {
                self.logger.error(format!(
                    "Restaurant not found for order: {}",
                    order.restaurant_id
                ));
            }
        } else {
            self.logger
                .error(format!("Order not found: {}", order.order_id));
        }
    }
}

impl Actor for Storage {
    type Context = Context<Self>;
}

/// Handles requests for the minimum log index currently stored.
impl Handler<GetMinLogIndex> for Storage {
    type Result = u64;

    fn handle(&mut self, _msg: GetMinLogIndex, _ctx: &mut Self::Context) -> Self::Result {
        self.min_persistent_log_index
    }
}

/// Handles requests for all log entries from a given index.
impl Handler<GetLogsFromIndex> for Storage {
    type Result = MessageResult<GetLogsFromIndex>;

    fn handle(&mut self, msg: GetLogsFromIndex, _ctx: &mut Self::Context) -> Self::Result {
        let mut logs = HashMap::new();
        for (id, log) in &self.storage_updates {
            if *id >= msg.index {
                logs.insert(*id, log.clone());
            }
        }
        MessageResult(logs)
    }
}

/// Applies a batch of storage updates, updating the log and state accordingly.
impl Handler<ApplyStorageUpdates> for Storage {
    type Result = ();

    fn handle(&mut self, msg: ApplyStorageUpdates, ctx: &mut Self::Context) -> Self::Result {
        let is_leader = msg.is_leader;
        let incoming_updates = msg.updates;

        if is_leader {
            // Si es el líder, elimina todas las operaciones recibidas de su registro.
            for (log_id, _) in &incoming_updates {
                self.storage_updates.remove(log_id);
                self.min_persistent_log_index = *log_id + 1;
            }
        } else {
            // Si NO es el líder:
            // 1. Elimina de su registro las operaciones que están en el registro pero NO en el mensaje recibido.
            let current_ids: Vec<u64> = self.storage_updates.keys().cloned().collect();
            let incoming_ids: std::collections::HashSet<u64> =
                incoming_updates.iter().map(|(id, _)| *id).collect();

            for id in current_ids.iter() {
                if !incoming_ids.contains(id) {
                    self.storage_updates.remove(id);
                    self.min_persistent_log_index = *id + 1;
                }
            }

            // 2. Aplica y guarda en su registro las operaciones que están en el mensaje recibido pero NO en el registro.
            for (id, update) in incoming_updates {
                match self.storage_updates.entry(id) {
                    Entry::Occupied(_) => {}
                    Entry::Vacant(entry) => {
                        // Aplica la operación
                        ctx.address().do_send(update.clone());

                        // Guarda la operación en el registro
                        entry.insert(update);
                    }
                }
            }
        }
        println!("MIN_LOG_INDEX: {}", self.min_persistent_log_index);
    }
}

/// Applies a single storage log message by dispatching it to the appropriate handler.
impl Handler<StorageLogMessage> for Storage {
    type Result = ();

    fn handle(&mut self, msg: StorageLogMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            StorageLogMessage::AddClient(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::AddRestaurant(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::AddDelivery(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::AddOrder(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::InsertAcceptedDelivery(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::AddAuthorizedOrderToRestaurant(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::AddPendingOrderToRestaurant(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::RemoveAcceptedDeliveries(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::RemoveClient(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::RemoveRestaurant(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::RemoveDelivery(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::RemoveAuthorizedOrderToRestaurant(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::RemovePendingOrderToRestaurant(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::RemoveOrder(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::SetDeliveryPosition(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::SetCurrentClientToDelivery(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::SetCurrentOrderToDelivery(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::SetDeliveryStatus(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::SetDeliveryToOrder(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::SetOrderStatus(msg) => {
                ctx.address().do_send(msg);
            }
            StorageLogMessage::SetOrderExpectedTime(msg) => {
                ctx.address().do_send(msg);
            }
        }
    }
}

/// Handles requests for a full snapshot of the storage state.
impl Handler<GetAllStorage> for Storage {
    type Result = MessageResult<GetAllStorage>;

    fn handle(&mut self, _msg: GetAllStorage, _ctx: &mut Self::Context) -> Self::Result {
        // Enviar toda la información del storage al coordinator.
        let snapshot = Snapshot {
            clients: self.clients.clone(),
            restaurants: self.restaurants.clone(),
            deliverys: self.deliverys.clone(),
            orders: self.orders.clone(),
            accepted_deliveries: self.accepted_deliveries.clone(),
            next_log_id: self.next_log_id,
            min_persistent_log_index: self.min_persistent_log_index,
        };
        self.logger.info("Snapshot sent to coordinator manager.");
        MessageResult(snapshot)
    }
}

/// Updates the storage state from a received snapshot.
impl Handler<StorageSnapshot> for Storage {
    type Result = ();

    fn handle(&mut self, msg: StorageSnapshot, _ctx: &mut Self::Context) -> Self::Result {
        // Por cada elemento que viene en el snapshot, lo piso en el storage.
        let snapshot = msg.snapshot.clone();

        for (client_id, client) in snapshot.clients {
            self.clients.insert(client_id, client);
        }
        for (restaurant_id, restaurant) in snapshot.restaurants {
            self.restaurants.insert(restaurant_id, restaurant);
        }
        for (delivery_id, delivery) in snapshot.deliverys {
            self.deliverys.insert(delivery_id, delivery);
        }
        for (order_id, order) in snapshot.orders {
            self.orders.insert(order_id, order);
        }
        for (order_id, delivery_id) in snapshot.accepted_deliveries {
            self.accepted_deliveries
                .insert(order_id, delivery_id);
        }
        self.next_log_id = snapshot.next_log_id;
        self.min_persistent_log_index = snapshot.min_persistent_log_index;
        self.logger
            .info("Storage snapshot updated from coordinator.");
    }
}

// --------------- ADD ------------------ //

/// Handles adding a new client to storage and logs the operation.
impl Handler<AddClient> for Storage {
    type Result = ();

    fn handle(&mut self, msg: AddClient, _ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Client added: {}", msg.client.client_id));
        self.clients
            .insert(msg.client.client_id.clone(), msg.client.clone());
        self.add_to_log(StorageLogMessage::AddClient(msg.clone()));
    }
}

/// Handles adding a new restaurant to storage and logs the operation.
impl Handler<AddRestaurant> for Storage {
    type Result = ();

    fn handle(&mut self, msg: AddRestaurant, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Restaurant added: {}",
            msg.restaurant.restaurant_id
        ));
        self.restaurants
            .insert(msg.restaurant.restaurant_id.clone(), msg.restaurant.clone());
        self.add_to_log(StorageLogMessage::AddRestaurant(msg.clone()));
    }
}

/// Handles adding a new delivery agent to storage and logs the operation.
impl Handler<AddDelivery> for Storage {
    type Result = ();

    fn handle(&mut self, msg: AddDelivery, _ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Delivery added: {}", msg.delivery.delivery_id));
        self.deliverys
            .insert(msg.delivery.delivery_id.clone(), msg.delivery.clone());
        self.add_to_log(StorageLogMessage::AddDelivery(msg.clone()));
    }
}

/// Handles adding a new order to storage and logs the operation.
impl Handler<AddOrder> for Storage {
    type Result = ();

    fn handle(&mut self, msg: AddOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Order added: {}", msg.order.order_id));
        self.orders.insert(msg.order.order_id, msg.order.clone());
        if let Some(client) = self.clients.get_mut(&msg.order.client_id) {
            client.client_order = Some(msg.order.clone());
        } else {
            self.logger.error(format!(
                "Client not found for order: {}",
                msg.order.client_id
            ));
        }
        self.add_to_log(StorageLogMessage::AddOrder(msg.clone()));
    }
}

/// Handles adding an accepted delivery for an order.
impl Handler<AddOrderAccepted> for Storage {
    type Result = ();

    fn handle(&mut self, msg: AddOrderAccepted, ctx: &mut Self::Context) -> Self::Result {
        if let Some(order) = self.orders.get_mut(&msg.order.order_id) {
            if order.status != OrderStatus::ReadyForDelivery {
                msg.addr.do_send(DeliveryNoNeeded {
                    order: msg.order.clone(),
                    delivery_info: msg.delivery.clone(),
                });
            }

            if self.accepted_deliveries.contains_key(&order.order_id) {
                // Ya existe una delivery aceptada para esta orden.
                self.logger.info(format!(
                    "Delivery already accepted for order: {}",
                    msg.order.order_id
                ));

                msg.addr.do_send(DeliveryNoNeeded {
                    order: msg.order.clone(),
                    delivery_info: msg.delivery.clone(),
                });

                // // Agrego la delivery a accepted_deliveries.
                // ctx.address().do_send(InsertAcceptedDelivery {
                //     order_id: msg.order.order_id,
                //     delivery_id: msg.delivery.delivery_id.clone(),
                // });
            } else if self.accepted_deliveries.contains_value(&msg.delivery.delivery_id) {
                // Ya existe una delivery aceptada con el mismo delivery_id.
                self.logger.info(format!(
                    "Delivery has already accepted order with ID: {}",
                    msg.order.order_id
                ));

                // Reenviar el mensaje al address contenida en el mensaje
                msg.addr.do_send(DeliveryNoNeeded {
                    order: msg.order.clone(),
                    delivery_info: msg.delivery.clone(),
                });
            
            } else {
                // No existe un delivery aceptada para esta orden.
                self.logger.info(format!(
                    "Adding accepted delivery for order: {}",
                    msg.order.order_id
                ));

                // Agrego la orden y el delivery a accepted_deliveries.
                self.handle(InsertAcceptedDelivery {
                    order_id: msg.order.order_id,
                    delivery_id: msg.delivery.delivery_id.clone(),
                }, ctx);

                // Reenviar el mensaje al address contenida en el mensaje
                msg.addr.do_send(DeliveryAvailable {
                    order: msg.order.clone(),
                    delivery_info: msg.delivery.clone(),
                });
            }
        } else {
            self.logger.error(format!(
                "Order not found for accepted delivery: {}",
                msg.order.order_id
            ));
            // Si la orden no está en el storage, no se puede aceptar la delivery.
            msg.addr.do_send(DeliveryNoNeeded {
                order: msg.order,
                delivery_info: msg.delivery,
            });
        }
    }
}

/// Handles inserting an accepted delivery for an order.
impl Handler<InsertAcceptedDelivery> for Storage {
    type Result = ();

    fn handle(&mut self, msg: InsertAcceptedDelivery, _ctx: &mut Self::Context) -> Self::Result {
        self.accepted_deliveries
            .insert(msg.order_id.clone(), msg.delivery_id.clone());
        self.add_to_log(StorageLogMessage::InsertAcceptedDelivery(msg.clone()));
    }
}

/// Handles adding an authorized order to a restaurant.
impl Handler<AddAuthorizedOrderToRestaurant> for Storage {
    type Result = ();

    fn handle(
        &mut self,
        msg: AddAuthorizedOrderToRestaurant,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        if let Some(restaurant) = self.restaurants.get_mut(&msg.restaurant_id) {
            if let Some(order) = self.orders.get(&msg.order.order_id) {
                restaurant.authorized_orders.insert(order.clone());
            } else {
                self.logger
                    .error(format!("Order not found: {}", msg.order.order_id));
            }
        } else {
            self.logger.error(format!(
                "Restaurant not found for order: {}",
                msg.restaurant_id
            ));
        }
        self.add_to_log(StorageLogMessage::AddAuthorizedOrderToRestaurant(
            msg.clone(),
        ));
    }
}

/// Handles adding a pending order to a restaurant.
impl Handler<AddPendingOrderToRestaurant> for Storage {
    type Result = ();

    fn handle(
        &mut self,
        msg: AddPendingOrderToRestaurant,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        if let Some(restaurant) = self.restaurants.get_mut(&msg.restaurant_id) {
            if let Some(order) = self.orders.get(&msg.order.order_id) {
                // TODO: Ver si hay que eliminar la orden de authorized_orders acá
                restaurant.authorized_orders.remove(order);
                restaurant.pending_orders.insert(order.clone());
            } else {
                self.logger
                    .error(format!("Order not found: {}", msg.order.order_id));
            }
        } else {
            self.logger.error(format!(
                "Restaurant not found for order: {}",
                msg.restaurant_id
            ));
        }
        self.add_to_log(StorageLogMessage::AddPendingOrderToRestaurant(msg.clone()));
    }
}

/// Handles the completion of a delivery assignment, removing accepted deliveries and notifying the address.
impl Handler<FinishDeliveryAssignment> for Storage {
    type Result = ();

    fn handle(&mut self, msg: FinishDeliveryAssignment, ctx: &mut Self::Context) -> Self::Result {
        if let Some(delivery_id) = msg.order.delivery_id.clone() {
            self.handle(SetDeliveryToOrder {
                order: msg.order.clone(),
                delivery_id: delivery_id.clone(),
            }, ctx);
            self.handle(SetOrderStatus {
                order: msg.order.clone(),
                order_status: OrderStatus::Delivering,
            }, ctx);

            self.handle(SetCurrentOrderToDelivery {
                delivery_id: delivery_id.clone(),
                order: msg.order.clone(),
            }, ctx);
            self.handle(SetCurrentClientToDelivery {
                delivery_id: delivery_id.clone(),
                client_id: msg.order.client_id.clone(),
            }, ctx);
            self.handle(RemovePendingOrderToRestaurant {
                order: msg.order.clone(),
                restaurant_id: msg.order.restaurant_id.clone(),
            }, ctx);
        } else {
            self.logger.error("No delivery ID found in order.");
            return;
        }

        self.handle(RemoveAcceptedDeliveries {
            order_id: msg.order.order_id,
        }, ctx);

        // let mut rejected_deliveries: HashMap<String, DeliveryDTO> = HashMap::new();
        // if let Some(deliveries) = self.accepted_deliveries.get(&msg.order.order_id) {
        //     for delivery_id in deliveries {
        //         if Some(delivery_id) == order.delivery_id.as_ref() {
        //             continue;
        //         }
        //         if let Some(delivery) = self.deliverys.get(delivery_id) {
        //             rejected_deliveries.insert(delivery_id.clone(), delivery.clone());
        //         } else {
        //             logger.warn(format!("Delivery not found for id: {}", delivery_id));
        //         }
        //     }
        // }

        // ctx.address().do_send(RemoveAcceptedDeliveries {
        //     order_id: order.order_id,
        // });
        // for (_delivery_id, delivery) in rejected_deliveries {
        //     addr.do_send(DeliveryNoNeeded {
        //         order: order.clone(),
        //         delivery_info: delivery.clone(),
        //     });
        // }
    }
}

/// Handles the removal of accepted deliveries for a specific order.
impl Handler<RemoveAcceptedDeliveries> for Storage {
    type Result = ();

    fn handle(&mut self, msg: RemoveAcceptedDeliveries, _ctx: &mut Self::Context) -> Self::Result {
        self.add_to_log(StorageLogMessage::RemoveAcceptedDeliveries(msg.clone()));
        self.accepted_deliveries.remove_by_key(&msg.order_id);
    }
}

/// Handles requests to get a client by ID.
impl Handler<GetClient> for Storage {
    type Result = MessageResult<GetClient>;

    fn handle(&mut self, msg: GetClient, _ctx: &mut Self::Context) -> Self::Result {
        MessageResult(self.clients.get(&msg.client_id).cloned())
    }
}

/// Handles requests to get a restaurant by ID.
impl Handler<GetRestaurant> for Storage {
    type Result = MessageResult<GetRestaurant>;

    fn handle(&mut self, msg: GetRestaurant, _ctx: &mut Self::Context) -> Self::Result {
        MessageResult(self.restaurants.get(&msg.restaurant_id).cloned())
    }
}

/// Handles requests to get a delivery by ID.
impl Handler<GetDelivery> for Storage {
    type Result = MessageResult<GetDelivery>;

    fn handle(&mut self, msg: GetDelivery, _ctx: &mut Self::Context) -> Self::Result {
        MessageResult(self.deliverys.get(&msg.delivery_id).cloned())
    }
}

/// Handles requests to get an order by ID.
impl Handler<GetOrder> for Storage {
    type Result = MessageResult<GetOrder>;

    fn handle(&mut self, msg: GetOrder, _ctx: &mut Self::Context) -> Self::Result {
        MessageResult(self.orders.get(&msg.order_id).cloned())
    }
}

// --------------- REMOVES ------------------ //

impl Handler<RemoveUser> for Storage {
    type Result = ();

    fn handle(&mut self, msg: RemoveUser, ctx: &mut Self::Context) -> Self::Result {
        if self.clients.contains_key(&msg.user_id) {
            self.handle(RemoveClient {
                client_id: msg.user_id,
            }, ctx);
        } else if self.restaurants.contains_key(&msg.user_id) {
            self.handle(RemoveRestaurant {
                restaurant_id: msg.user_id,
            }, ctx);
        } else if self.deliverys.contains_key(&msg.user_id) {
            self.handle(RemoveDelivery {
                delivery_id: msg.user_id,
            }, ctx);
        } else {
            self.logger
                .error(format!("User not found: {}", msg.user_id));
        }
    }
}


/// Handles removing a client from storage and logs the operation.
impl Handler<RemoveClient> for Storage {
    type Result = ();

    fn handle(&mut self, msg: RemoveClient, _ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Client removed: {}", msg.client_id));
        self.clients.remove(&msg.client_id);
        self.add_to_log(StorageLogMessage::RemoveClient(msg.clone()));
    }
}

/// Handles removing a restaurant from storage and logs the operation.
impl Handler<RemoveRestaurant> for Storage {
    type Result = ();

    fn handle(&mut self, msg: RemoveRestaurant, _ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Restaurant removed: {}", msg.restaurant_id));
        self.restaurants.remove(&msg.restaurant_id);
        self.add_to_log(StorageLogMessage::RemoveRestaurant(msg.clone()));
        // TODO: ver como hacer cascade con las órdenes asociadas a este restaurante.
    }
}

/// Handles removing a delivery agent from storage and logs the operation.
impl Handler<RemoveDelivery> for Storage {
    type Result = ();

    fn handle(&mut self, msg: RemoveDelivery, _ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Delivery removed: {}", msg.delivery_id));
        self.deliverys.remove(&msg.delivery_id);
        self.add_to_log(StorageLogMessage::RemoveDelivery(msg.clone()));
    }
}

/// Handles removing an authorized order from a restaurant.
impl Handler<RemoveAuthorizedOrderToRestaurant> for Storage {
    type Result = ();

    fn handle(
        &mut self,
        msg: RemoveAuthorizedOrderToRestaurant,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        if let Some(restaurant) = self.restaurants.get_mut(&msg.restaurant_id) {
            if let Some(order) = self.orders.get(&msg.order.order_id) {
                restaurant.authorized_orders.remove(order);
            } else {
                self.logger
                    .error(format!("Order not found: {}", msg.order.order_id));
            }
        } else {
            self.logger.error(format!(
                "Restaurant not found for order: {}",
                msg.restaurant_id
            ));
        }
        self.add_to_log(StorageLogMessage::RemoveAuthorizedOrderToRestaurant(
            msg.clone(),
        ));
    }
}

/// Handles removing a pending order from a restaurant.
impl Handler<RemovePendingOrderToRestaurant> for Storage {
    type Result = ();

    fn handle(
        &mut self,
        msg: RemovePendingOrderToRestaurant,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        if let Some(restaurant) = self.restaurants.get_mut(&msg.restaurant_id) {
            if let Some(order) = self.orders.get(&msg.order.order_id) {
                restaurant.pending_orders.remove(order);
            } else {
                self.logger
                    .error(format!("Order not found: {}", msg.order.order_id));
            }
        } else {
            self.logger.error(format!(
                "Restaurant not found for order: {}",
                msg.restaurant_id
            ));
        }
        self.add_to_log(StorageLogMessage::RemovePendingOrderToRestaurant(
            msg.clone(),
        ));
    }
}

/// Handles removing an order from storage and logs the operation.
impl Handler<RemoveOrder> for Storage {
    type Result = ();

    fn handle(&mut self, msg: RemoveOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Order removed: {}", msg.order.order_id));
        if let Some(order) = self.orders.remove(&msg.order.order_id) {
            // Limpiar la orden del cliente
            if let Some(client) = self.clients.get_mut(&order.client_id) {
                client.client_order = None;
            } else {
                self.logger
                    .warn(format!("Client not found for order: {}", order.client_id));
            }

            // Limpiar la orden de los pedidos del restaurante
            if let Some(restaurant) = self.restaurants.get_mut(&order.restaurant_id) {
                restaurant.pending_orders.remove(&order);
                restaurant.authorized_orders.remove(&order);
            } else {
                self.logger.warn(format!(
                    "Restaurant not found for order: {}",
                    order.restaurant_id
                ));
            }

            // Limpiar la orden del delivery si corresponde
            if let Some(delivery_id) = &order.delivery_id {
                if let Some(delivery) = self.deliverys.get_mut(delivery_id) {
                    if let Some(current_order) = &delivery.current_order {
                        if current_order.order_id == order.order_id {
                            delivery.current_order = None;
                        }
                    }
                } else {
                    self.logger
                        .warn(format!("Delivery not found for order: {}", delivery_id));
                }
            }
            self.clients.remove(&msg.order.client_id);
        } else {
            self.logger
                .error(format!("Order not found: {}", msg.order.order_id));
        }
        self.add_to_log(StorageLogMessage::RemoveOrder(msg.clone()));
    }
}

// --------------- SETTERS ------------------ //

/// Handles setting the delivery position for a specific delivery.
impl Handler<SetDeliveryPosition> for Storage {
    type Result = ();

    fn handle(&mut self, msg: SetDeliveryPosition, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(delivery) = self.deliverys.get_mut(&msg.delivery_id) {
            delivery.delivery_position = msg.position;
            self.logger
                .info(format!("Delivery position updated: {}", msg.delivery_id));
        } else {
            self.logger
                .error(format!("Delivery not found: {}", msg.delivery_id));
        }
        self.add_to_log(StorageLogMessage::SetDeliveryPosition(msg.clone()));
    }
}

/// Handles setting the current client for a specific delivery.
impl Handler<SetCurrentClientToDelivery> for Storage {
    type Result = ();

    fn handle(
        &mut self,
        msg: SetCurrentClientToDelivery,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        if let Some(delivery) = self.deliverys.get_mut(&msg.delivery_id) {
            delivery.current_client_id = Some(msg.client_id.clone());
            self.logger.info(format!(
                "Current client set for delivery: {}",
                msg.delivery_id
            ));
        } else {
            self.logger
                .error(format!("Delivery not found: {}", msg.delivery_id));
        }
        self.add_to_log(StorageLogMessage::SetCurrentClientToDelivery(msg.clone()));
    }
}

/// Handles setting the current order for a specific delivery.
impl Handler<SetCurrentOrderToDelivery> for Storage {
    type Result = ();

    fn handle(&mut self, msg: SetCurrentOrderToDelivery, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(delivery) = self.deliverys.get_mut(&msg.delivery_id) {
            // Obtener la orden del id pasado en el mensaje
            if let Some(order) = self.orders.get(&msg.order.order_id) {
                delivery.current_order = Some(order.clone());
                self.logger.info(format!(
                    "Current order set for delivery: {}",
                    msg.delivery_id
                ));
            } else {
                self.logger
                    .error(format!("Order not found: {}", msg.order.order_id));
            }
        } else {
            self.logger
                .error(format!("Delivery not found: {}", msg.delivery_id));
        }
        self.add_to_log(StorageLogMessage::SetCurrentOrderToDelivery(msg.clone()));
    }
}

/// Handles setting the delivery status for a specific delivery.
impl Handler<SetDeliveryStatus> for Storage {
    type Result = ();

    fn handle(&mut self, msg: SetDeliveryStatus, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(delivery) = self.deliverys.get_mut(&msg.delivery_id) {
            delivery.status = msg.delivery_status;
            self.logger
                .info(format!("Delivery status updated: {}", msg.delivery_id));
        } else {
            self.logger
                .error(format!("Delivery not found: {}", msg.delivery_id));
        }
        self.add_to_log(StorageLogMessage::SetDeliveryStatus(msg));
    }
}

/// Handles setting a delivery to an order, updating the order's delivery ID.
impl Handler<SetDeliveryToOrder> for Storage {
    type Result = ();

    fn handle(&mut self, msg: SetDeliveryToOrder, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(order) = self.orders.get_mut(&msg.order.order_id) {
            order.delivery_id = Some(msg.delivery_id.clone());
            let order_clone = order.clone();
            self.update_associated_order(&order_clone);
        } else {
            self.logger
                .error(format!("Order not found: {}", msg.order.order_id));
        }
        self.add_to_log(StorageLogMessage::SetDeliveryToOrder(msg.clone()));
    }
}

/// Handles updating the status of an order.
impl Handler<SetOrderStatus> for Storage {
    type Result = ();

    fn handle(&mut self, msg: SetOrderStatus, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(order) = self.orders.get_mut(&msg.order.order_id) {
            order.status = msg.order_status.clone();
            let order_clone = order.clone();
            self.update_associated_order(&order_clone);
        } else {
            self.logger
                .error(format!("Order not found: {}", msg.order.order_id));
        }
        self.add_to_log(StorageLogMessage::SetOrderStatus(msg.clone()));
    }
}

impl Handler<SetOrderExpectedTime> for Storage {
    type Result = ();

    fn handle(&mut self, msg: SetOrderExpectedTime, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(order) = self.orders.get_mut(&msg.order_id) {
            order.expected_delivery_time = msg.expected_time;
            let order_clone = order.clone();
            self.update_associated_order(&order_clone);
        } else {
            self.logger
                .error(format!("Order not found: {}", msg.order_id));
        }
        self.add_to_log(StorageLogMessage::SetOrderExpectedTime(msg.clone()));
    }
}

/// Handles requests to get all restaurants in storage
impl Handler<GetRestaurants> for Storage {
    type Result = MessageResult<GetRestaurants>;

    fn handle(&mut self, _msg: GetRestaurants, _ctx: &mut Self::Context) -> Self::Result {
        let restaurants: Vec<RestaurantDTO> = self.restaurants.values().cloned().collect();
        MessageResult(restaurants)
    }
}

/// Handles requests to get all restaurant information (ID and position).
impl Handler<GetAllRestaurantsInfo> for Storage {
    type Result = MessageResult<GetAllRestaurantsInfo>;

    fn handle(&mut self, _msg: GetAllRestaurantsInfo, _ctx: &mut Self::Context) -> Self::Result {
        let restaurants: Vec<RestaurantInfo> = self
            .restaurants
            .values()
            .map(|r| RestaurantInfo {
                id: r.restaurant_id.clone(),
                position: r.restaurant_position,
            })
            .collect();
        MessageResult(restaurants)
    }
}

/// Handles requests to get all deliveries in storage
impl Handler<GetDeliveries> for Storage {
    type Result = MessageResult<GetDeliveries>;

    fn handle(&mut self, _msg: GetDeliveries, _ctx: &mut Self::Context) -> Self::Result {
        let deliveries: Vec<DeliveryDTO> = self.deliverys.values().cloned().collect();
        println!(
            "[STORAGEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE][DEBUG] Deliveries en storage: {:?}",
            deliveries
        );
        MessageResult(deliveries)
    }
}

/// Handles requests to get all available deliveries in storage.
impl Handler<GetAllAvailableDeliveries> for Storage {
    type Result = MessageResult<GetAllAvailableDeliveries>;

    fn handle(
        &mut self,
        _msg: GetAllAvailableDeliveries,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let available_deliveries: Vec<DeliveryDTO> = self
            .deliverys
            .values()
            .filter(|d| d.status == common::types::delivery_status::DeliveryStatus::Available)
            .cloned()
            .collect();
        MessageResult(available_deliveries)
    }
}
