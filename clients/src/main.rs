use std::net::SocketAddr;
use actix::prelude::*;
use common::constants;
use common::messages::shared_messages::StartRunning;
mod client;
use client::Client;
use tokio::signal::ctrl_c;

#[actix::main]
async fn main() -> std::io::Result<()> {
    let servers: Vec<SocketAddr> = (0..constants::NUM_COORDINATORS)
        .map(|i| {
            format!("127.0.0.1:{}", constants::BASE_PORT + i as u16)
                .parse()
                .expect("Dirección IP inválida")
        })
        .collect();

    let client = Client::new(servers).await; // Creo la structura del cliente
    let addr = client.start(); // Inicia el cliente como actor
    addr.do_send(StartRunning);
    // run async
    tokio::select! {
        _ = ctrl_c() => {
            println!("Ctrl-C received, shutting down...");
        }
    }
    Ok(())
}

