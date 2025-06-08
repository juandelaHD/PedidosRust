use actix::prelude::*;
use common::constants::{self, SUCCESS_PROBABILITY};
use common::messages::shared_messages::StartRunning;
use common::utils::get_rand_f32_tuple;
use std::env;
use std::net::SocketAddr;
mod delivery;
use delivery::Delivery;

use tokio::signal::ctrl_c;

#[actix::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: {} <delivery_id>", args[0]);
        std::process::exit(1);
    }

    let id = args[1].clone();

    let servers: Vec<SocketAddr> = (0..constants::NUM_COORDINATORS)
        .map(|i| {
            format!("127.0.0.1:{}", constants::BASE_PORT + i as u16)
                .parse()
                .expect("Dirección IP inválida")
        })
        .collect();

    let position = get_rand_f32_tuple();

    println!(
        "Creando delivery con ID: {}, posición: {:?}, éxito?: {}",
        id, position, SUCCESS_PROBABILITY
    );

    let delivery = Delivery::new(servers.clone(), id, position, SUCCESS_PROBABILITY).await;
    let addr = delivery.start();

    addr.do_send(StartRunning);

    tokio::select! {
        _ = ctrl_c() => {
            println!("Ctrl-C recibido, apagando...");
        }
    }

    Ok(())
}
