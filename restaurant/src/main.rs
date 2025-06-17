use actix::prelude::*;
use common::constants::{BASE_PORT, NUM_COORDINATORS, SERVER_IP_ADDRESS, SUCCESS_PROBABILITY};
use common::types::restaurant_info::RestaurantInfo;
use common::utils::get_rand_f32_tuple;
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

    let servers: Vec<SocketAddr> = (0..NUM_COORDINATORS)
        .map(|i| {
            format!("{}:{}", SERVER_IP_ADDRESS, BASE_PORT + i as u16)
                .parse()
                .expect("Dirección IP inválida")
        })
        .collect();

    let id = args[1].clone();
    let position = get_rand_f32_tuple();

    println!(
        "Creando restaurant con ID: {}, posición: {:?}, éxito?: {}",
        id, position, SUCCESS_PROBABILITY
    );

    let restaurant = Restaurant::new(
        RestaurantInfo { id, position },
        SUCCESS_PROBABILITY,
        servers,
    )
    .await;

    restaurant.start();

    tokio::select! {
        _ = ctrl_c() => {
            println!("Ctrl-C recibido, apagando...");
        }
    }

    Ok(())
}
