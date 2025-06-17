use crate::network::peer_types::PeerType;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

/// Intenta conectarse a todos los `SocketAddr` proporcionados y devuelve
/// un `HashMap` de direcciones a streams abiertos exitosamente.
pub async fn connect_to_all(
    servers: Vec<SocketAddr>,
    peer_type: PeerType,
) -> HashMap<SocketAddr, TcpStream> {
    let mut connections = HashMap::new();

    for addr in servers {
        match TcpStream::connect(addr).await {
            Ok(mut stream) => {
                // Enviar el byte que indica el tipo de peer
                let type_byte = [peer_type.to_u8()];
                if let Err(e) = stream.write_all(&type_byte).await {
                    eprintln!("Failed to send peer type to {}: {}", addr, e);
                    continue;
                }

                stream
                    .write_all(b"\n")
                    .await
                    .unwrap_or_else(|_| eprintln!("Failed to send newline to {}", addr));

                println!("Connected to {} as {:?}", addr, peer_type);
                connections.insert(addr, stream);
            }
            Err(e) => {
                eprintln!("Failed to connect to {}: {}", addr, e);
            }
        }
    }

    connections
}

pub async fn connect_some(servers: Vec<SocketAddr>, peer_type: PeerType) -> Option<TcpStream> {
    for addr in servers {
        match try_to_connect(addr).await {
            Some(mut stream) => {
                // Enviar el byte que indica el tipo de peer
                let type_byte = [peer_type.to_u8()];
                if let Err(e) = stream.write_all(&type_byte).await {
                    eprintln!("Failed to send peer type to {}: {}", addr, e);
                    continue;
                }

                stream
                    .write_all(b"\n")
                    .await
                    .unwrap_or_else(|_| eprintln!("Failed to send newline to {}", addr));

                println!("Connected to {} as {:?}", addr, peer_type);
                return Some(stream);
            }
            None => {
                eprintln!("Failed to connect to {}", addr);
            }
        }
    }

    None
}

/// Intenta conectarse a un solo `SocketAddr` y devuelve un `Option` con el `TcpStream` si tuvo éxito.
pub async fn try_to_connect(server_addr: SocketAddr) -> Option<TcpStream> {
    match TcpStream::connect(server_addr).await {
        Ok(stream) => Some(stream),
        Err(_) => None,
    }
}
