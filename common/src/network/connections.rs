use crate::network::peer_types::PeerType;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::{Duration, timeout};

pub async fn connect_to_all(
    servers: Vec<SocketAddr>,
    peer_type: PeerType,
) -> HashMap<SocketAddr, TcpStream> {
    let mut connections = HashMap::new();

    for addr in servers {
        match TcpStream::connect(addr).await {
            Ok(mut stream) => {
                // Enviar solo el byte del tipo de peer
                let type_byte = [peer_type.to_u8()];
                if let Err(e) = stream.write_all(&type_byte).await {
                    eprintln!("Failed to send peer type to {}: {}", addr, e);
                    continue;
                }
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
                // Enviar solo el byte del tipo de peer
                let type_byte = [peer_type.to_u8()];
                if let Err(e) = stream.write_all(&type_byte).await {
                    eprintln!("Failed to send peer type to {}: {}", addr, e);
                    continue;
                }
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

pub async fn connect_one(addr: SocketAddr, peer_type: PeerType) -> Option<TcpStream> {
    if let Some(mut stream) = try_to_connect(addr).await {
        // Enviar solo el byte del tipo de peer
        let type_byte = [peer_type.to_u8()];
        if let Err(e) = stream.write_all(&type_byte).await {
            eprintln!("Failed to send peer type to {}: {}", addr, e);
            return None;
        }
        println!("Connected to {} as {:?}", addr, peer_type);
        Some(stream)
    } else {
        eprintln!("Failed to connect to {}", addr);
        None
    }
}

async fn try_to_connect(server_addr: SocketAddr) -> Option<TcpStream> {
    TcpStream::connect(server_addr).await.ok()
}

pub async fn reconnect(servers: Vec<SocketAddr>, peer_type: PeerType) -> Option<TcpStream> {
    for addr in servers {
        println!("Trying to connect to {}", addr);
        match timeout(Duration::from_secs(2), TcpStream::connect(addr)).await {
            Ok(Ok(mut stream)) => {
                // Enviar solo el byte del tipo de peer
                let type_byte = [peer_type.to_u8()];
                if let Err(e) = stream.write_all(&type_byte).await {
                    println!("Failed to send peer type to {}: {}", addr, e);
                    continue;
                }
                println!("Successfully connected to {}", addr);
                return Some(stream);
            }
            Ok(Err(e)) => {
                println!("Failed to connect to {}: {}", addr, e);
                continue;
            }
            Err(_) => {
                println!("Timeout while connecting to {}", addr);
                continue;
            }
        }
    }
    None
}
