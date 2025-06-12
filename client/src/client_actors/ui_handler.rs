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
        // Simula la interacción con el usuario para seleccionar un restaurante y un plato
        self.logger.info("Por favor, seleccione un restaurante:");

        for (i, restaurant) in possible_restaurants.iter().enumerate() {
            self.logger.info(format!(
                "{}: {} - {}",
                i + 1,
                restaurant.name,
                restaurant.id
            ));
        }

        // Selección de restaurante válida
        let selected_index = loop {
            print!("Ingrese el número del restaurante deseado: ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            if let Err(e) = io::stdin().read_line(&mut input) {
                println!("Error leyendo entrada: {}", e);
                continue;
            }

            match input.trim().parse::<usize>() {
                Ok(num) if num >= 1 && num <= possible_restaurants.len() => break num - 1,
                _ => println!(
                    "Por favor ingrese un número válido entre 1 y {}",
                    possible_restaurants.len()
                ),
            }
        };

        let selected_restaurant = &possible_restaurants[selected_index];
        println!(
            "Seleccionó: {} (ID: {})",
            selected_restaurant.name, selected_restaurant.id
        );

        // Ingreso del nombre del plato
        let dish_name = loop {
            print!("Ingrese el nombre del plato que desea pedir: ");
            io::stdout().flush().unwrap();

            let mut dish_input = String::new();
            if let Err(e) = io::stdin().read_line(&mut dish_input) {
                println!("Error leyendo entrada: {}", e);
                continue;
            }

            let dish = dish_input.trim();
            if !dish.is_empty() {
                break dish.to_string();
            } else {
                println!("El nombre del plato no puede estar vacío.");
            }
        };

        println!("Ha seleccionado el plato: {}", dish_name);
        // Retorna el restaurante seleccionado y el plato
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
        self.logger.info(format!(
            "Seleccionando restaurantes cercanos: {:?}",
            msg.nearby_restaurants
        ));
        if msg.nearby_restaurants.is_empty() {
            self.logger
                .warn("No hay restaurantes cercanos disponibles.");
            // TODO: Handlear el caso de no tener restaurantes cercanos
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
