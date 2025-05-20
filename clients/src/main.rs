use std::net::SocketAddr;
use actix::prelude::*;
use common::constants;
mod client;
use client::Client;

#[actix::main]
async fn main() -> std::io::Result<()> {
    let servers: Vec<SocketAddr> = (0..constants::NUM_COORDINATORS)
        .map(|i| format!("127.0.0.1:{}", constants::BASE_PORT + i as u16).parse().unwrap())
        .collect();

    Client::new(servers).start();

    Ok(())
}

