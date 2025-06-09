use actix::prelude::*;
use common::constants::{SERVER_IP_ADDRESS, SERVER_PORT};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal::ctrl_c;
use tokio::time::{Duration, sleep};

use common::logger::Logger;

use server::server_acceptor::acceptor::Acceptor;
use server::server_actors::server_actor::Coordinator;

#[actix::main]
async fn main() {
    println!("Server is starting...");
    let main_logger = Arc::new(Logger::new("Main"));
    let server_addr_res = format!("{}:{}", SERVER_IP_ADDRESS, SERVER_PORT).parse::<SocketAddr>();

    match server_addr_res {
        Ok(server_addr) => {
            main_logger.info(format!(
                "Server address parsed successfully: {}",
                server_addr
            ));
            // Seguir con la inicialización...
            let coordinator = Coordinator::new(server_addr);
            let acceptor = Acceptor::new(server_addr, coordinator.clone());
            acceptor.start();
        }
        Err(e) => {
            main_logger.error(format!("Failed to parse server address: {}", e));
            // Quizá salir o intentar fallback
            return;
        }
    }

    main_logger.info("Starting the delivery service...");

    tokio::select! {
        _ = ctrl_c() => {
            println!("Ctrl-C recibido, apagando...");
        }
    }
}
