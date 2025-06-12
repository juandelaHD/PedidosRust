use actix::prelude::*;
use common::constants::{BASE_PORT, NUM_COORDINATORS, SERVER_IP_ADDRESS, SUCCESS_PROBABILITY};
use common::messages::shared_messages::StartRunning;
use common::utils::get_rand_f32_tuple;
use std::env;
use std::net::SocketAddr;
mod kitchen;
mod restaurant;
use restaurant::Restaurant;

//mod kitchen; // si querés también inicializar la Kitchen

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
            format!("{}:{}", SERVER_IP_ADDRESS, BASE_PORT + i as u16)
                .parse()
                .expect("Dirección IP inválida")
        })
        .collect();

    let position = get_rand_f32_tuple();

    println!(
        "Creando restaurant con ID: {}, posición: {:?}, éxito?: {}",
        id, position, SUCCESS_PROBABILITY
    );

    let restaurant = Restaurant::new(servers.clone(), id, position, SUCCESS_PROBABILITY).await;

    let addr = restaurant.start();

    addr.do_send(StartRunning);

    tokio::select! {
        _ = ctrl_c() => {
            println!("Ctrl-C recibido, apagando...");
        }
    }

    Ok(())
}
