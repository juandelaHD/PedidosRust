use crate::client_actors::client::Client;
use crate::messages::internal_messages::{SelectNearbyRestaurants, SendThisOrder};
use actix::prelude::*;
use common::logger::Logger;
use common::types::restaurant_info::RestaurantInfo;
use std::io::Write;

/// The `UIHandler` actor is responsible for managing the user interface interactions
/// in the client application. It prompts the user to select a restaurant and dish,
/// and communicates the user's choices to the `Client` actor.
pub struct UIHandler {
    /// Address of the `Client` actor to send user selections to.
    pub client: Addr<Client>,
    /// Logger for UI-related messages and errors.
    pub logger: Logger,
}

impl UIHandler {
    /// Creates a new `UIHandler` instance.
    ///
    /// ## Arguments
    ///
    /// * `client` - Address of the `Client` actor.
    /// * `logger` - Logger instance for UI messages.
    pub fn new(client: Addr<Client>, logger: Logger) -> Self {
        UIHandler { client, logger }
    }
}

impl Actor for UIHandler {
    type Context = Context<Self>;
}

/// Handles the `SelectNearbyRestaurants` message.
///
/// Extracts the list of nearby restaurants and dishes.
///
/// Prompts the user to select a restaurant and dish, then sending the
/// selection to the `Client` actor.
impl Handler<SelectNearbyRestaurants> for UIHandler {
    type Result = ();

    fn handle(&mut self, msg: SelectNearbyRestaurants, ctx: &mut Self::Context) {
        if msg.nearby_restaurants.is_empty() {
            self.logger
                .warn("No nearby restaurants found. Please try again later.");
            return;
        }
        let logger = self.logger.clone();
        let restaurants = msg.nearby_restaurants.clone();
        let addr = ctx.address();

        // Spawn blocking para no trabar el actor
        actix::spawn(async move {
            let (restaurant_id, dish_name) =
                tokio::task::spawn_blocking(move || ask_user_order_blocking(&logger, restaurants))
                    .await
                    .unwrap();

            addr.do_send(UserOrderResult {
                restaurant_id,
                dish_name,
            });
        });
    }
}

pub struct UserOrderResult {
    pub restaurant_id: String,
    pub dish_name: String,
}

impl Message for UserOrderResult {
    type Result = ();
}

fn ask_user_order_blocking(
    logger: &Logger,
    possible_restaurants: Vec<RestaurantInfo>,
) -> (String, String) {
    let selected_index = loop {
        logger.info("Select a restaurant by number:");
        for (i, restaurant) in possible_restaurants.iter().enumerate() {
            logger.info(format!("{}: {}", i + 1, restaurant.id));
        }
        std::io::stdout().flush().unwrap();

        let mut input = String::new();
        if let Err(e) = std::io::stdin().read_line(&mut input) {
            logger.error(format!(
                "Error while reading input: {}. Please try again.",
                e
            ));
            continue;
        }

        match input.trim().parse::<usize>() {
            Ok(num) if num >= 1 && num <= possible_restaurants.len() => break num - 1,
            _ => {
                logger.warn(
                    "Invalid selection. Please enter a number corresponding to a restaurant.",
                );
                continue;
            }
        }
    };

    let selected_restaurant = &possible_restaurants[selected_index];

    // Ingreso del nombre del plato
    let dish_name = loop {
        logger.info("Please enter the name of the dish you want to order:");
        std::io::stdout().flush().unwrap();

        let mut dish_input = String::new();
        if let Err(e) = std::io::stdin().read_line(&mut dish_input) {
            logger.error(format!(
                "Error while reading dish input: {}. Please try again.",
                e
            ));
            continue;
        }

        let dish = dish_input.trim();
        if !dish.is_empty() {
            break dish.to_string();
        } else {
            logger.warn("Dish name cannot be empty. Please enter a valid dish name.");
            continue;
        }
    };

    logger.info(format!(
        "You selected restaurant: {} and dish: {}",
        selected_restaurant.id, dish_name
    ));
    (selected_restaurant.id.clone(), dish_name)
}

impl Handler<UserOrderResult> for UIHandler {
    type Result = ();

    fn handle(&mut self, msg: UserOrderResult, _ctx: &mut Self::Context) {
        self.client.do_send(SendThisOrder {
            selected_restaurant: msg.restaurant_id,
            selected_dish: msg.dish_name,
        });
    }
}
