use crate::network::peer_types::PeerType;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

/// Attempts to connect to all provided socket addresses as the specified peer type,
/// returning a map of successfully connected addresses to their TCP streams.
///
/// ## Arguments
/// - `servers`: A list of server socket addresses to connect to.
/// - `peer_type`: The type of this peer (sent to the server).
///
/// ## Returns
/// A `HashMap` mapping each successfully connected address to its `TcpStream`.
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

/// Attempts to connect to the first available server in the list as the specified peer type.
///
/// ## Arguments
/// - `servers`: A list of server socket addresses to try.
/// - `peer_type`: The type of this peer (sent to the server).
///
/// ## Returns
/// An `Option<TcpStream>` with the first successful connection, or `None` if all fail.
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

/// Attempts to connect to a single server address.
///
/// ## Arguments
/// - `server_addr`: The address to connect to.
///
/// ## Returns
/// An `Option<TcpStream>` if the connection succeeds.
pub async fn connect_one(addr: SocketAddr, peer_type: PeerType) -> Option<TcpStream> {
    if let Some(mut stream) = try_to_connect(addr).await {
        // Enviar el byte que indica el tipo de peer
        let type_byte = [peer_type.to_u8()];
        if let Err(e) = stream.write_all(&type_byte).await {
            eprintln!("Failed to send peer type to {}: {}", addr, e);
            return None;
        }

        if let Err(e) = stream.write_all(b"\n").await {
            eprintln!("Failed to send newline to {}: {}", addr, e);
            return None;
        }

        println!("Connected to {} as {:?}", addr, peer_type);
        Some(stream)
    } else {
        eprintln!("Failed to connect to {}", addr);
        None
    }
}

/// Attempts to connect to a server address and returns the `TcpStream` if successful.
///
/// ## Arguments
/// - `server_addr`: The address to connect to.
///
/// ## Returns
/// An `Option<TcpStream>` if the connection succeeds, or `None` if it fails.
pub async fn try_to_connect(server_addr: SocketAddr) -> Option<TcpStream> {
    match TcpStream::connect(server_addr).await {
        Ok(stream) => Some(stream),
        Err(_) => None,
    }
}

/// Attempts to reconnect to any of the provided servers as the specified peer type.
///
/// ## Arguments
/// - `servers`: A list of server addresses.
/// - `peer_type`: The type of this peer.
///
/// ## Returns
/// An `Option<TcpStream>` with a new connection if successful.
pub async fn reconnect(servers: Vec<SocketAddr>, peer_type: PeerType) -> Option<TcpStream> {
    let new_stream = connect_some(servers, peer_type).await;
    new_stream
}
