use actix::prelude::*;
use common::constants::{
    BASE_PORT, DELIVERY_SUCCESS_PROBABILITY, NUM_COORDINATORS, SERVER_IP_ADDRESS,
};
use common::utils::{get_rand_f32_tuple, print_welcome_message};
use delivery::delivery_actors::delivery::Delivery;
use std::env;
use std::net::SocketAddr;

use tokio::signal::ctrl_c;

#[actix::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <delivery_id>", args[0]);
        std::process::exit(1);
    }

    let id = args[1].clone();
    let servers: Vec<SocketAddr> = (0..NUM_COORDINATORS)
        .map(|i| {
            format!("{}:{}", SERVER_IP_ADDRESS, BASE_PORT + i)
                .parse()
                .expect("Invalid IP address")
        })
        .collect();

    let position = get_rand_f32_tuple();

    print_welcome_message();

    let delivery = Delivery::new(servers.clone(), id, position, DELIVERY_SUCCESS_PROBABILITY).await;
    delivery.start();

    tokio::select! {
        _ = ctrl_c() => {
            actix::System::current().stop();
        }
    }

    Ok(())
}
