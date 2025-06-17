use crate::server_actors::coordinator::Coordinator;
use crate::server_actors::storage::Storage;
use actix::prelude::*;
use colored::Color;
use common::constants::NEARBY_RADIUS;
use common::logger::Logger;
use common::messages::CancelOrder;
use common::messages::NearbyRestaurants;
use common::messages::RequestNearbyRestaurants;
use common::messages::internal_messages::GetAllRestaurantsInfo;
use common::types::dtos::OrderDTO;
use common::types::order_status::OrderStatus;
use common::types::restaurant_info::RestaurantInfo;
use common::utils::calculate_distance;

pub struct NearbyRestaurantsService {
    // Cache local de repartidores disponibles con su ubicación.
    pub storage_addr: Addr<Storage>,
    pub coordinator_addr: Addr<Coordinator>,
    pub logger: Logger,
}

impl NearbyRestaurantsService {
    pub fn new(storage_addr: Addr<Storage>, coordinator_addr: Addr<Coordinator>) -> Self {
        let logger = Logger::new("Nearby Restaurants Service", Color::Green);
        NearbyRestaurantsService {
            storage_addr,
            coordinator_addr,
            logger,
        }
    }

    fn get_nearby_restaurants(
        &self,
        available_restaurants: Vec<RestaurantInfo>,
        location: (f32, f32),
    ) -> Vec<RestaurantInfo> {
        available_restaurants
            .into_iter()
            .filter(|restaurant| {
                let distance = calculate_distance(restaurant.position, location);
                distance <= NEARBY_RADIUS
            })
            .collect()
    }
}

impl Actor for NearbyRestaurantsService {
    type Context = Context<Self>;
}

impl Handler<RequestNearbyRestaurants> for NearbyRestaurantsService {
    type Result = ();

    fn handle(&mut self, msg: RequestNearbyRestaurants, ctx: &mut Self::Context) -> Self::Result {
        let storage_addr = self.storage_addr.clone();
        let coordinator_addr = self.coordinator_addr.clone();
        let logger = self.logger.clone();
        let client = msg.client.clone();
        let location = msg.client.client_position;
        let get_nearby_restaurants = NearbyRestaurantsService::get_nearby_restaurants;

        let order_dummy_cancelled = OrderDTO {
            order_id: 0,
            client_id: msg.client.client_id,
            dish_name: "None".to_string(),
            restaurant_id: "None".to_string(),
            status: OrderStatus::Cancelled,
            delivery_id: None,
            client_position: msg.client.client_position,
            expected_delivery_time: 0,
            time_stamp: std::time::SystemTime::now(),
        };

        storage_addr
            .send(GetAllRestaurantsInfo)
            .into_actor(self)
            .map(move |res, act, _ctx| match res {
                Ok(restaurants) => {
                    if restaurants.is_empty() {
                        logger.warn("Retrieved no restaurants from storage.");
                        coordinator_addr.do_send(CancelOrder {
                            order: order_dummy_cancelled,
                        });
                    } else {
                        logger.info(format!(
                            "Retrieved {} restaurants from storage.",
                            restaurants.len()
                        ));
                        let nearby: Vec<RestaurantInfo> =
                            get_nearby_restaurants(act, restaurants.clone(), location);

                        if nearby.is_empty() {
                            logger.warn("No nearby restaurants found.");
                            coordinator_addr.do_send(NearbyRestaurants {
                                client,
                                restaurants: restaurants.clone(),
                            });
                        } else {
                            logger.info(format!(
                                "Found {} nearby restaurants for client at position: {:?}",
                                nearby.len(),
                                location
                            ));
                            coordinator_addr.do_send(NearbyRestaurants {
                                client,
                                restaurants: nearby,
                            });
                        }
                    }
                }
                Err(_) => {
                    logger.error("Error retrieving restaurants from storage.");
                    coordinator_addr.do_send(NearbyRestaurants {
                        client,
                        restaurants: Vec::new(),
                    });
                }
            })
            .wait(ctx);
    }
}
