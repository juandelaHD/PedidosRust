use std::net::SocketAddr;
use common::constants;
mod client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut servers = Vec::new();

    for i in 0..constants::NUM_COORDINATORS {
        let port = constants::BASE_PORT + i as u16;
        let server: SocketAddr = format!("127.0.0.1:{}", port).parse()?;
        servers.push(server);
    }

    let mut client = client::Client::new(servers).await;
    client.run().await;

    Ok(())
}
