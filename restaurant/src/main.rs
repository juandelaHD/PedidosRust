use actix::prelude::*;
use common::constants::{self, SUCCESS_PROBABILITY};
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

    let servers: Vec<SocketAddr> = (0..constants::NUM_COORDINATORS)
        .map(|i| {
            format!("127.0.0.1:{}", constants::BASE_PORT + i as u16)
                .parse()
                .expect("Dirección IP inválida")
        })
        .collect();

    let position = get_rand_f32_tuple();

    println!(
        "Creando restaurant con ID: {}, posición: {:?}, éxito?: {}",
        id, position, SUCCESS_PROBABILITY
    );

    // Si querés también inicializar Kitchen acá y pasarle la dirección al Restaurant:
    // let kitchen = Kitchen::new(...).start();

    // Restaurant necesita la dirección del Kitchen:
    // let kitchen_addr = todo!("Inicializá Kitchen y obtené su Addr");

    //let kitchen_addr: Option<Addr<Kitchen>> = None;

    let restaurant = Restaurant::new(
        servers.clone(),
        id,
        position,
        SUCCESS_PROBABILITY,
        //kitchen_addr,
    )
    .await;

    let addr = restaurant.start();

    //addr.do_send(StartRunning);

    tokio::select! {
        _ = ctrl_c() => {
            println!("Ctrl-C recibido, apagando...");
        }
    }

    Ok(())
}
