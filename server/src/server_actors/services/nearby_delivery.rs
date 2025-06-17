use common::messages::internal_messages::{GetDeliveries, RemoveOrder};
use crate::server_actors::coordinator::Coordinator;
use crate::server_actors::storage::Storage;
use actix::prelude::*;
use colored::Color;
use common::constants::NEARBY_RADIUS;
use common::logger::Logger;
use common::messages::CancelOrder;
use common::messages::coordinator_messages::NearbyDeliveries;
use common::messages::restaurant_messages::RequestNearbyDelivery;
use common::types::dtos::DeliveryDTO;
use common::utils::calculate_distance;

pub struct NearbyDeliveryService {
    pub coordinator_address: Addr<Coordinator>,
    pub storage_address: Addr<Storage>,
    pub logger: Logger,
}

impl NearbyDeliveryService {
    pub fn new(storage_address: Addr<Storage>, coordinator_address: Addr<Coordinator>) -> Self {
        let logger = Logger::new("Nearby Delivery Service", Color::Green);
        NearbyDeliveryService {
            coordinator_address,
            storage_address,
            logger,
        }
    }

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

    fn handle(&mut self, msg: RequestNearbyDelivery, ctx: &mut Context<Self>) {
        let coordinator_addr = self.coordinator_address.clone();
        let logger = self.logger.clone();
        let get_nearby_deliveries = NearbyDeliveryService::get_nearby_deliveries;
        let storage_addr = self.storage_address.clone();
        let restaurant = msg.restaurant_info.position;
        let order = msg.order;
        self.logger.info(format!(
            "Requesting nearby deliveries for order: {:?} at restaurant position: {:?}",
            order, restaurant
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
                                order
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
