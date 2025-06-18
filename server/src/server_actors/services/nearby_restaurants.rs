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

/// The `NearbyRestaurantsService` actor is responsible for handling requests
/// for nearby restaurants based on the client's location. It retrieves
/// available restaurants from the storage, filters them based on proximity,
/// and sends the results back to the coordinator.
///
/// ## Responsibilities:
/// - Retrieve all available restaurants from the storage.
/// - Filter restaurants based on the client's location and a predefined radius.
/// - Send the list of nearby restaurants back to the coordinator.
pub struct NearbyRestaurantsService {
    /// The address of the Storage actor to fetch restaurants from.
    pub storage_addr: Addr<Storage>,
    /// The address of the Coordinator actor to send messages to.
    pub coordinator_addr: Addr<Coordinator>,
    /// Logger instance for events
    pub logger: Logger,
}

impl NearbyRestaurantsService {
    /// Creates a new instance of `NearbyRestaurantService`.
    ///
    /// ## Arguments
    /// * `storage_address` - The address of the Storage actor.
    /// * `coordinator_address` - The address of the Coordinator actor.
    pub fn new(storage_addr: Addr<Storage>, coordinator_addr: Addr<Coordinator>) -> Self {
        let logger = Logger::new("Nearby Restaurants Service", Color::Green);
        NearbyRestaurantsService {
            storage_addr,
            coordinator_addr,
            logger,
        }
    }

    /// Filters the list of available restaurants to find those within a specified radius
    /// from the client's location.
    ///
    /// ## Arguments
    /// * `available_restaurants` - A vector of `RestaurantInfo` containing all available restaurants.
    /// * `location` - A tuple representing the client's location as (latitude, longitude).
    ///
    /// ## Returns
    /// A vector of `RestaurantInfo` containing only the restaurants that are within the specified radius.
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

    /// Handles the `RequestNearbyRestaurants` message by retrieving all restaurants from storage,
    /// filtering them based on the client's location, and sending the results back to the coordinator.
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
