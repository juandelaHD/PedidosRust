use crate::client_actors::client::Client;
use crate::messages::internal_messages::{SelectNearbyRestaurants, SendThisOrder};
use actix::prelude::*;
use common::logger::Logger;
use common::types::restaurant_info::RestaurantInfo;
use std::io::{self, Write};

/// Actor UIHandler: Interfaz humano-sistema
pub struct UIHandler {
    /// Canal de envío hacia el actor `Client`
    pub client: Addr<Client>,
    pub logger: Logger,
}

impl UIHandler {
    pub fn new(client: Addr<Client>, logger: Logger) -> Self {
        UIHandler { client, logger }
    }

    fn ask_user_order(
        &self,
        _ctx: &mut Context<Self>,
        possible_restaurants: Vec<RestaurantInfo>,
    ) -> (String, String) {
        let selected_index = loop {
            self.logger.info("\nSelect a restaurant by number:");
            for (i, restaurant) in possible_restaurants.iter().enumerate() {
                self.logger.info(format!("{}: {}", i + 1, restaurant.id));
            }
            io::stdout().flush().unwrap();

            let mut input = String::new();
            if let Err(e) = io::stdin().read_line(&mut input) {
                self.logger.error(format!(
                    "Error while reading input: {}. Please try again.",
                    e
                ));
                continue;
            }

            match input.trim().parse::<usize>() {
                Ok(num) if num >= 1 && num <= possible_restaurants.len() => break num - 1,
                _ => {
                    self.logger.warn(
                        "Invalid selection. Please enter a number corresponding to a restaurant.",
                    );
                    continue;
                }
            }
        };

        let selected_restaurant = &possible_restaurants[selected_index];

        // Ingreso del nombre del plato
        let dish_name = loop {
            self.logger
                .info("Please enter the name of the dish you want to order:");
            io::stdout().flush().unwrap();

            let mut dish_input = String::new();
            if let Err(e) = io::stdin().read_line(&mut dish_input) {
                self.logger.error(format!(
                    "Error while reading dish input: {}. Please try again.",
                    e
                ));
                continue;
            }

            let dish = dish_input.trim();
            if !dish.is_empty() {
                break dish.to_string();
            } else {
                self.logger
                    .warn("Dish name cannot be empty. Please enter a valid dish name.");
                continue;
            }
        };

        self.logger.info(format!(
            "You selected restaurant: {} and dish: {}",
            selected_restaurant.id, dish_name
        ));
        (selected_restaurant.id.clone(), dish_name)
    }
}

impl Actor for UIHandler {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        self.logger.info("UIHandler iniciado!");
    }
}

impl Handler<SelectNearbyRestaurants> for UIHandler {
    type Result = ();

    fn handle(&mut self, msg: SelectNearbyRestaurants, ctx: &mut Self::Context) {
        if msg.nearby_restaurants.is_empty() {
            self.logger
                .warn("No nearby restaurants found. Please try again later.");
            return;
        }
        // Llama a la función para pedir al usuario que seleccione un restaurante y un plato
        let (restaurant_id, dish_name) = self.ask_user_order(ctx, msg.nearby_restaurants);
        // Envía el pedido al actor Client
        self.client.do_send(SendThisOrder {
            selected_restaurant: restaurant_id,
            selected_dish: dish_name,
        });
    }
}
