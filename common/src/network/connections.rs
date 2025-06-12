use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::TcpStream;

/// Intenta conectarse a todos los `SocketAddr` proporcionados y devuelve
/// un `HashMap` de direcciones a streams abiertos exitosamente.
pub async fn connect_to_all(servers: Vec<SocketAddr>) -> HashMap<SocketAddr, TcpStream> {
    let mut connections = HashMap::new();

    for addr in servers {
        match TcpStream::connect(addr).await {
            Ok(stream) => {
                println!("Connected to {}", addr);
                connections.insert(addr, stream);
            }
            Err(e) => {
                eprintln!("Failed to connect to {}: {}", addr, e);
            }
        }
    }

    connections
}

/// Intenta conectarse a un solo `SocketAddr` y devuelve un `Option` con el `TcpStream` si tuvo éxito.
pub async fn connect(server_addr: SocketAddr) -> Option<TcpStream> {
    match TcpStream::connect(server_addr).await {
        Ok(stream) => Some(stream),
        Err(_) => None,
    }
}
