use crate::server_actors::coordinator::Coordinator;
use crate::server_actors::storage::Storage;
use actix::prelude::*;
use colored::Color;
use common::constants::NEARBY_RADIUS;
use common::logger::Logger;
use common::messages::CancelOrder;
use common::messages::coordinator_messages::NearbyDeliveries;
use common::messages::internal_messages::{GetDeliveries, RemoveOrder};
use common::messages::restaurant_messages::RequestNearbyDelivery;
use common::types::dtos::DeliveryDTO;
use common::utils::calculate_distance;

/// The `NearbyDeliveryService` Actor is responsible for tracking nearby delivery users suitable
/// for a given restaurant's order based on their geographical position.
///
/// ## Responsibilities
/// - Fetches available deliveries from the storage.
/// - Filters deliveries based on proximity to the restaurant's position.
/// - Sends the filtered list of nearby deliveries to the coordinator.
pub struct NearbyDeliveryService {
    /// The address of the Coordinator actor to send messages to.
    pub coordinator_address: Addr<Coordinator>,
    /// The address of the Storage actor to fetch deliveries from.
    pub storage_address: Addr<Storage>,
    /// Logger instance for events
    pub logger: Logger,
}

impl NearbyDeliveryService {
    /// Creates a new instance of `NearbyDeliveryService`.
    ///
    /// ## Arguments
    /// * `storage_address` - The address of the Storage actor.
    /// * `coordinator_address` - The address of the Coordinator actor.
    pub fn new(storage_address: Addr<Storage>, coordinator_address: Addr<Coordinator>) -> Self {
        let logger = Logger::new("Nearby Delivery Service", Color::Green);
        NearbyDeliveryService {
            coordinator_address,
            storage_address,
            logger,
        }
    }

    /// Filters the available deliveries to find those that are within a specified radius
    /// from the restaurant's position.
    ///
    /// ## Arguments
    /// * `available_deliveries` - A vector of `DeliveryDTO` containing all available deliveries.
    /// * `restaurant_pos` - A tuple representing the restaurant's geographical position (latitude, longitude).
    ///
    /// ## Returns
    /// A vector of `DeliveryDTO` containing only those deliveries that are within the specified radius.
    fn get_nearby_deliveries(
        &self,
        available_deliveries: Vec<DeliveryDTO>,
        restaurant_pos: (f32, f32),
    ) -> Vec<DeliveryDTO> {
        available_deliveries
            .into_iter()
            .filter(|delivery| {
                let distance = calculate_distance(delivery.delivery_position, restaurant_pos);
                distance <= NEARBY_RADIUS
            })
            .collect()
    }
}

impl Actor for NearbyDeliveryService {
    type Context = Context<Self>;
}

impl Handler<RequestNearbyDelivery> for NearbyDeliveryService {
    type Result = ();

    /// Handles the `RequestNearbyDelivery` message by fetching deliveries from storage,
    /// filtering them based on proximity to the restaurant's position, and sending the results
    /// to the Coordinator actor.
    fn handle(&mut self, msg: RequestNearbyDelivery, ctx: &mut Context<Self>) {
        let coordinator_addr = self.coordinator_address.clone();
        let logger = self.logger.clone();
        let get_nearby_deliveries = NearbyDeliveryService::get_nearby_deliveries;
        let storage_addr = self.storage_address.clone();
        let restaurant = msg.restaurant_info.position;
        let order = msg.order;
        self.logger.info(format!(
            "Requesting nearby deliveries for order: {:?} at restaurant position: {:?}",
            order.order_id, restaurant
        ));
        self.storage_address
            .send(GetDeliveries)
            .into_actor(self)
            .map(move |res, act, _ctx| match res {
                Ok(deliveries) => {
                    if deliveries.is_empty() {
                        logger.warn("Retrived  no deliveries from storage.");
                        coordinator_addr.do_send(CancelOrder {
                            order: order.clone(),
                        });
                        storage_addr.do_send(RemoveOrder { order });
                    } else {
                        logger.info(format!(
                            "Retrieved {} deliveries from storage.",
                            deliveries.len()
                        ));
                        let nearby: Vec<DeliveryDTO> =
                            get_nearby_deliveries(act, deliveries.clone(), restaurant);
                        if nearby.is_empty() {
                            logger.warn(
                                "No nearby deliveries found for the order. Sending all deliveries.",
                            );
                            coordinator_addr.do_send(NearbyDeliveries {
                                order,
                                deliveries: deliveries.clone(),
                            });
                        } else {
                            logger.info(format!(
                                "Found {} nearby deliveries for order: {:?}",
                                nearby.len(),
                                order.order_id
                            ));
                            coordinator_addr.do_send(NearbyDeliveries {
                                order,
                                deliveries: nearby,
                            });
                        }
                    }
                }
                Err(_) => {
                    logger.error("Error retrieving deliveries from storage.");
                    coordinator_addr.do_send(NearbyDeliveries {
                        order,
                        deliveries: Vec::new(),
                    });
                }
            })
            .wait(ctx);
    }
}
