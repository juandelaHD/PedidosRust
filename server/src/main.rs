use std::net::SocketAddr;
mod server_acceptor;
mod server_actors;
use crate::server_acceptor::acceptor::Acceptor;

#[actix::main]
async fn main() -> std::io::Result<()> {
    let port = 8080;
    let addr: SocketAddr = format!("127.0.0.1:{}", port)
        .parse()
        .expect("Error al parsear la dirección IP");

    //let peers: Vec<SocketAddr> = (0..5)
    //    .map(|i| format!("127:0.0.1:{}", 8081 + i as u16).parse().unwrap())
    //    .collect();
    let acceptor = Acceptor::new(addr);
    acceptor.start().await?;

    Ok(())
}