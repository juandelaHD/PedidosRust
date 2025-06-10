use actix::Actor;
use client::client_actors::client::Client;
use common::constants::{BASE_PORT, NUM_COORDINATORS, SERVER_IP_ADDRESS, SERVER_PORT};
use common::logger::Logger;
use common::messages::shared_messages::StartRunning;
use common::utils::get_rand_f32_tuple;
use std::io::{self, Write};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::signal::ctrl_c;

#[actix::main]
async fn main() -> io::Result<()> {
    let main_logger = Arc::new(Logger::new("Main"));

    let servers: Vec<SocketAddr> = (0..NUM_COORDINATORS)
        .map(|i| {
            format!("{}:{}", SERVER_IP_ADDRESS, BASE_PORT + i as u16)
                .parse()
                .expect("Dirección IP inválida")
        })
        .collect();

    let position = get_rand_f32_tuple();

    const ID_DUMMY: &str = "ABCD";

    println!(
        "Creando client con ID: {}, posición: {:?}",
        ID_DUMMY, position
    );

    let client = Client::new(servers.clone(), ID_DUMMY.to_string(), position).await;
    let addr = client.start();

    addr.do_send(StartRunning);

    tokio::select! {
        _ = ctrl_c() => {
            println!("Ctrl-C recibido, apagando...");
        }
    }
    Ok(())
}
