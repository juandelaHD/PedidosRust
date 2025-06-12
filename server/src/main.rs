use actix::prelude::*;
use common::constants::SERVER_IP_ADDRESS;
use common::constants::{BASE_PORT, NUM_COORDINATORS};
use common::logger::Logger;
use server::server_acceptor::acceptor::Acceptor;
use server::server_actors::server_actor::Coordinator;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal::ctrl_c;

#[actix::main]
async fn main() {
    println!("Server is starting...");
    let main_logger = Arc::new(Logger::new("Main"));

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

    main_logger.info(format!("Starting server at: {}", my_addr));

    // Construir la lista completa de ring_nodes
    let ip = SERVER_IP_ADDRESS.parse().unwrap();
    let ring_nodes = (0..NUM_COORDINATORS)
        .map(|i| SocketAddr::new(ip, BASE_PORT + i as u16))
        .collect::<Vec<_>>();

    // Inicializar Coordinator
    let coordinator = Coordinator::new(my_addr, ring_nodes.clone(), main_logger.clone());
    let acceptor = Acceptor::new(my_addr, coordinator.clone());

    // Arrancar Acceptor
    acceptor.start();

    main_logger.info("Server fully started and ready.");

    // Esperar señal de apagado
    tokio::select! {
        _ = ctrl_c() => {
            println!("Ctrl-C recibido, apagando...");
        }
    }
}
