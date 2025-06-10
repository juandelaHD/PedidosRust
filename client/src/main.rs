use actix::Actor;
use client::client_actors::client::Client;
use common::constants::{BASE_PORT, NUM_COORDINATORS, SERVER_IP_ADDRESS};
use common::messages::shared_messages::StartRunning;
use common::utils::get_rand_f32_tuple;
use std::io::{self};
use std::net::SocketAddr;
use tokio::signal::ctrl_c;
use std::env;

#[actix::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: {} <client_id>", args[0]);
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
        "Creando client con ID: {}, posición: {:?}",
        id, position
    );

    let client = Client::new(servers.clone(), id, position).await;
    let addr = client.start();

    addr.do_send(StartRunning);

    tokio::select! {
        _ = ctrl_c() => {
            println!("Ctrl-C recibido, apagando...");
        }
    }
    Ok(())
}
