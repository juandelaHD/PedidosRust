use actix::prelude::*;
use common::constants::SERVER_IP_ADDRESS;
use common::constants::{BASE_PORT, NUM_COORDINATORS};
use server::server_acceptor::acceptor::Acceptor;
use server::server_actors::coordinator::Coordinator;
use std::env;
use std::net::SocketAddr;
use tokio::signal::ctrl_c;

#[actix::main]
async fn main() {
    // Permitir pasar el puerto como argumento: ejemplo => cargo run -- 8081
    let args: Vec<String> = env::args().collect();
    let port = if args.len() > 1 {
        args[1].parse::<u16>().expect("Invalid port number")
    } else {
        BASE_PORT // default a 8080
    };

    let my_addr = format!("{}:{}", SERVER_IP_ADDRESS, port)
        .parse::<SocketAddr>()
        .expect("Failed to parse server address");
    // Construir la lista completa de ring_nodes
    let ip: std::net::IpAddr = SERVER_IP_ADDRESS.parse().unwrap();
    let ring_nodes = (0..NUM_COORDINATORS)
        .map(|i| SocketAddr::new(ip, BASE_PORT + i as u16))
        .filter(|addr| *addr != my_addr)
        .collect::<Vec<SocketAddr>>();
    // Iniciar el Coordinator
    let coordinator = Coordinator::new(my_addr, ring_nodes.clone()).await;
    let coordinator_addr = coordinator.start();

    // Iniciar el Acceptor (le pasamos ambos)
    Acceptor::new(my_addr, coordinator_addr.clone()).start();


    // Esperar señal de apagado
    tokio::select! {
        _ = ctrl_c() => {
            println!("Ctrl-C recibido, apagando...");
        }
    }
}
