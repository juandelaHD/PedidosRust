use actix::prelude::*;
use common::logger::Logger;
use common::types::dtos::DeliveryDTO;
use common::messages::coordinator_messages::{NearbyDeliveries};
use common::messages::restaurant_messages::{RequestNearbyDelivery};
use common::utils::calculate_distance;
use common::constants::NEARBY_RADIUS;
use crate::messages::internal_messages::GetDeliveries;
use crate::server_actors::coordinator::Coordinator;
use crate::server_actors::storage::Storage;

pub struct NearbyDeliveryService {
    pub coordinator_address: Addr<Coordinator>,
    pub storage_address: Addr<Storage>,
    pub logger: Logger,
}

impl NearbyDeliveryService {
    pub fn new(coordinator_address: Addr<Coordinator>, storage_address: Addr<Storage>) -> Self {
        let logger = Logger::new("NearbyDeliveryService");        NearbyDeliveryService {
            coordinator_address,
            storage_address,
            logger,
        }
    }

    fn get_nearby_deliveries(&self, available_deliveries: Vec<DeliveryDTO>, restaurant_pos: (f32, f32)) -> Vec<DeliveryDTO> {
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

        let restaurant = msg.restaurant_info.position;
        let order = msg.order;

        self.storage_address
            .send(GetDeliveries)
            .into_actor(self)
            .map(move |res, act, _ctx| match res {
                Ok(deliveries) => {
                    let nearby: Vec<DeliveryDTO> =
                        get_nearby_deliveries(act, deliveries, restaurant);

                    coordinator_addr.do_send(NearbyDeliveries {
                        order,                        
                        deliveries: nearby,
                    });
                }
                Err(_) => {
                    logger.error("Error al obtener restaurantes del storage.");
                    coordinator_addr.do_send(NearbyDeliveries{
                        order,
                        deliveries: Vec::new(),
                    });
                }
            })
            .wait(ctx);
    }
}
