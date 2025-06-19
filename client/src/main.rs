use actix::Actor;
use client::client_actors::client::Client;
use common::constants::{BASE_PORT, NUM_COORDINATORS, SERVER_IP_ADDRESS};
use common::utils::{get_rand_f32_tuple, print_welcome_message};
use std::env;
use std::net::SocketAddr;
use tokio::signal::ctrl_c;

#[actix::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: {} <client_id>", args[0]);
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

    print_welcome_message();

    let position = get_rand_f32_tuple();

    let client = Client::new(servers.clone(), id, position).await;
    client.start();

    tokio::select! {
        _ = ctrl_c() => {
            println!("Ctrl-C recibido, apagando...");
            actix::System::current().stop();
        }
    }
    Ok(())
}
