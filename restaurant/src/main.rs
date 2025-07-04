use actix::prelude::*;
use common::constants::{
    BASE_PORT, NUM_COORDINATORS, RESTAURANT_SUCCESS_PROBABILITY, SERVER_IP_ADDRESS,
};
use common::types::restaurant_info::RestaurantInfo;
use common::utils::{get_rand_f32_tuple, print_welcome_message};
use restaurant::restaurant_actors::restaurant::Restaurant;
use std::env;
use std::net::SocketAddr;
use tokio::signal::ctrl_c;

#[actix::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: {} <restaurant_id>", args[0]);
        std::process::exit(1);
    }

    let id = args[1].clone();

    let servers: Vec<SocketAddr> = (0..NUM_COORDINATORS)
        .map(|i| {
            format!("{}:{}", SERVER_IP_ADDRESS, BASE_PORT + i)
                .parse()
                .expect("Dirección IP inválida")
        })
        .collect();

    let position = get_rand_f32_tuple();

    print_welcome_message();

    let restaurant = Restaurant::new(
        RestaurantInfo { id, position },
        RESTAURANT_SUCCESS_PROBABILITY,
        servers,
    )
    .await;

    restaurant.start();

    tokio::select! {
        _ = ctrl_c() => {
            actix::System::current().stop();
        }
    }

    Ok(())
}
