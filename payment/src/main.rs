use actix::prelude::*;
use common::constants::{PAYMENT_GATEWAY_PORT, PAYMENT_SUCCESS_PROBABILITY, SERVER_IP_ADDRESS};
use std::net::SocketAddr;
use tokio::signal::ctrl_c;
mod payment;
mod payment_acceptor;
use payment::PaymentGateway;
use payment_acceptor::PaymentAcceptor;

#[actix::main]
async fn main() {
    let my_addr = format!("{}:{}", SERVER_IP_ADDRESS, PAYMENT_GATEWAY_PORT)
        .parse::<SocketAddr>()
        .expect("Failed to parse server address");

    // Iniciar el PaymentGateway
    let payment_gateway = PaymentGateway::new(PAYMENT_SUCCESS_PROBABILITY);
    let payment_gateway_addr = payment_gateway.start();

    // Iniciar el Acceptor
    PaymentAcceptor::new(my_addr, payment_gateway_addr.clone()).start();

    // Esperar señal de apagado
    tokio::select! {
        _ = ctrl_c() => {
            println!("Ctrl-C recibido, apagando...");
        }
    }
}
