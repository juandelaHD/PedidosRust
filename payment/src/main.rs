use std::env;
use std::net::SocketAddr;
mod payment;
use payment::PaymentGatewayActor;

#[actix::main]
async fn main() -> std::io::Result<()> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8090".to_string());

    let socket_addr: SocketAddr = addr.parse().expect("Invalid address");

    PaymentGatewayActor::start(socket_addr).await?;

    Ok(())
}
