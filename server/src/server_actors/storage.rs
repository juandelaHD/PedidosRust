use crate::messages::internal_messages::{
    AddAuthorizedOrderToRestaurant, AddClient, AddDelivery, AddOrder, AddPendingOrderToRestaurant,
    AddRestaurant, GetAllAvailableDeliveries, GetAllRestaurantsInfo, GetClient, GetDeliveries,
    GetDelivery, GetOrder, GetRestaurant, GetRestaurants, RemoveAuthorizedOrderToRestaurant,
    RemoveClient, RemoveDelivery, RemoveOrder, RemovePendingOrderToRestaurant, RemoveRestaurant,
    SetCurrentClientToDelivery, SetCurrentOrderToDelivery, SetDeliveryPosition, SetDeliveryStatus,
    SetDeliveryToOrder, SetOrderStatus, StorageLogMessage,
};
use crate::server_actors::coordinator::Coordinator;
use actix::prelude::*;
use common::logger::Logger;
use common::types::{
    dtos::{ClientDTO, DeliveryDTO, OrderDTO, RestaurantDTO},
    restaurant_info::RestaurantInfo,
};
use std::collections::HashMap;

pub struct Storage {
    /// Diccionario con información sobre clientes.
    pub clients: HashMap<String, ClientDTO>,
    /// Diccionario con información sobre restaurantes.
    pub restaurants: HashMap<String, RestaurantDTO>,
    /// Diccionario con información sobre deliverys.
    pub deliverys: HashMap<String, DeliveryDTO>,
    /// Diccionario de órdenes.
    pub orders: HashMap<u64, OrderDTO>,
    /// Lista de actualizaciones del storage.
    pub storage_updates: HashMap<u64, StorageLogMessage>,
    /// Comunicador asociado al `Coordinator`.
    pub coordinator: Addr<Coordinator>,
    /// Logger para registrar eventos en el storage.
    pub logger: Logger,
}

impl Storage {
    pub fn new(coordinator: Addr<Coordinator>) -> Self {
        Self {
            clients: HashMap::new(),
            restaurants: HashMap::new(),
            deliverys: HashMap::new(),
            orders: HashMap::new(),
            storage_updates: HashMap::new(),
            coordinator,
            logger: Logger::new("Storage".to_string()),
        }
    }
}

impl Actor for Storage {
    type Context = Context<Self>;
}

impl Handler<AddClient> for Storage {
    type Result = ();

    fn handle(&mut self, msg: AddClient, _ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Client added: {}", msg.client.client_id));
        self.clients
            .insert(msg.client.client_id.clone(), msg.client);
    }
}

impl Handler<AddRestaurant> for Storage {
    type Result = ();

    fn handle(&mut self, msg: AddRestaurant, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Restaurant added: {}",
            msg.restaurant.restaurant_id
        ));
        self.restaurants
            .insert(msg.restaurant.restaurant_id.clone(), msg.restaurant);
    }
}
impl Handler<AddDelivery> for Storage {
    type Result = ();

    fn handle(&mut self, msg: AddDelivery, _ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Delivery added: {}", msg.delivery.delivery_id));
        self.deliverys
            .insert(msg.delivery.delivery_id.clone(), msg.delivery);
    }
}

impl Handler<AddOrder> for Storage {
    type Result = ();

    fn handle(&mut self, msg: AddOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Order added: {}", msg.order.order_id));
        self.orders.insert(msg.order.order_id, msg.order.clone());
        if let Some(client) = self.clients.get_mut(&msg.order.client_id) {
            client.client_order = Some(msg.order);
        } else {
            self.logger.error(format!(
                "Client not found for order: {}",
                msg.order.client_id
            ));
        }
    }
}

impl Handler<GetClient> for Storage {
    type Result = MessageResult<GetClient>;

    fn handle(&mut self, msg: GetClient, _ctx: &mut Self::Context) -> Self::Result {
        MessageResult(self.clients.get(&msg.restaurant_id).cloned())
    }
}

impl Handler<GetRestaurant> for Storage {
    type Result = MessageResult<GetRestaurant>;

    fn handle(&mut self, msg: GetRestaurant, _ctx: &mut Self::Context) -> Self::Result {
        MessageResult(self.restaurants.get(&msg.restaurant_id).cloned())
    }
}

impl Handler<GetDelivery> for Storage {
    type Result = MessageResult<GetDelivery>;

    fn handle(&mut self, msg: GetDelivery, _ctx: &mut Self::Context) -> Self::Result {
        MessageResult(self.deliverys.get(&msg.delivery_id).cloned())
    }
}

impl Handler<GetOrder> for Storage {
    type Result = MessageResult<GetOrder>;

    fn handle(&mut self, msg: GetOrder, _ctx: &mut Self::Context) -> Self::Result {
        MessageResult(self.orders.get(&msg.order_id).cloned())
    }
}

impl Handler<RemoveClient> for Storage {
    type Result = ();

    fn handle(&mut self, msg: RemoveClient, _ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Client removed: {}", msg.client_id));
        // TODO: Ver si es necesario eliminar órdenes asociadas al cliente.
        if let Some(order) = self
            .clients
            .get(&msg.client_id)
            .and_then(|c| c.client_order.clone())
        {
            self.orders.remove(&order.order_id);
            self.logger
                .info(format!("Order removed for client: {}", msg.client_id));
        } else {
            self.logger
                .error(format!("No order found for client: {}", msg.client_id));
        }

        self.clients.remove(&msg.client_id);
    }
}

impl Handler<RemoveRestaurant> for Storage {
    type Result = ();

    fn handle(&mut self, msg: RemoveRestaurant, _ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Restaurant removed: {}", msg.restaurant_id));
        self.restaurants.remove(&msg.restaurant_id);
        // TODO: ver como hacer cascade con las órdenes asociadas a este restaurante.
    }
}

impl Handler<RemoveDelivery> for Storage {
    type Result = ();

    fn handle(&mut self, msg: RemoveDelivery, _ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Delivery removed: {}", msg.delivery_id));
        self.deliverys.remove(&msg.delivery_id);
    }
}

impl Handler<RemoveOrder> for Storage {
    type Result = ();

    fn handle(&mut self, msg: RemoveOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Order removed: {}", msg.order.order_id));
        if let Some(order) = self.orders.remove(&msg.order.order_id) {
            if let Some(client) = self.clients.get_mut(&order.client_id) {
                client.client_order = None;
            } else {
                self.logger
                    .error(format!("Client not found for order: {}", order.client_id));
            }
        } else {
            self.logger
                .error(format!("Order not found: {}", msg.order.order_id));
        }
    }
}

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
    }
}

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
                restaurant.authorized_orders.remove(&order);
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
    }
}

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
    }
}

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
    }
}

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
    }
}

impl Handler<SetCurrentClientToDelivery> for Storage {
    type Result = ();

    fn handle(
        &mut self,
        msg: SetCurrentClientToDelivery,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        if let Some(delivery) = self.deliverys.get_mut(&msg.delivery_id) {
            delivery.current_client_id = Some(msg.client_id);
            self.logger.info(format!(
                "Current client set for delivery: {}",
                msg.delivery_id
            ));
        } else {
            self.logger
                .error(format!("Delivery not found: {}", msg.delivery_id));
        }
    }
}

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
    }
}

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
    }
}

impl Handler<SetDeliveryToOrder> for Storage {
    type Result = ();

    fn handle(&mut self, msg: SetDeliveryToOrder, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(order) = self.orders.get_mut(&msg.order.order_id) {
            order.delivery_id = Some(msg.delivery_id);
            self.logger
                .info(format!("Delivery set for order: {}", msg.order.order_id));
        } else {
            self.logger
                .error(format!("Order not found: {}", msg.order.order_id));
        }
    }
}

impl Handler<SetOrderStatus> for Storage {
    type Result = ();

    fn handle(&mut self, msg: SetOrderStatus, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(order) = self.orders.get_mut(&msg.order.order_id) {
            order.status = msg.order_status;
            self.logger
                .info(format!("Order status updated: {}", msg.order.order_id));
        } else {
            self.logger
                .error(format!("Order not found: {}", msg.order.order_id));
        }
    }
}

impl Handler<GetRestaurants> for Storage {
    type Result = MessageResult<GetRestaurants>;

    fn handle(&mut self, _msg: GetRestaurants, _ctx: &mut Self::Context) -> Self::Result {
        let restaurants: Vec<RestaurantDTO> = self.restaurants.values().cloned().collect();
        MessageResult(restaurants)
    }
}

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

impl Handler<GetDeliveries> for Storage {
    type Result = MessageResult<GetDeliveries>;

    fn handle(&mut self, _msg: GetDeliveries, _ctx: &mut Self::Context) -> Self::Result {
        let deliveries: Vec<DeliveryDTO> = self.deliverys.values().cloned().collect();
        MessageResult(deliveries)
    }
}

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
