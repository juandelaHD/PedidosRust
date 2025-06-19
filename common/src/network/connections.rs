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
    match TcpStream::connect(server_addr).await {
        Ok(stream) => Some(stream),
        Err(_) => None,
    }
}

pub async fn reconnect(servers: Vec<SocketAddr>, peer_type: PeerType) -> Option<TcpStream> {
    println!(
        "[reconnect] Entrando a reconnect con servidores: {:?}",
        servers
    );
    for addr in servers {
        println!("[reconnect] Intentando conectar a {}", addr);
        match timeout(Duration::from_secs(2), TcpStream::connect(addr)).await {
            Ok(Ok(mut stream)) => {
                // Enviar solo el byte del tipo de peer
                let type_byte = [peer_type.to_u8()];
                if let Err(e) = stream.write_all(&type_byte).await {
                    println!(
                        "[reconnect] Falló el envío del tipo de peer a {}: {}",
                        addr, e
                    );
                    continue;
                }
                println!("[reconnect] Conectado exitosamente a {}", addr);
                return Some(stream);
            }
            Ok(Err(e)) => {
                println!("[reconnect] Falló la conexión a {}: {}", addr, e);
                continue;
            }
            Err(_) => {
                println!("[reconnect] Timeout al conectar a {}", addr);
                continue;
            }
        }
    }
    println!("[reconnect] No se pudo conectar a ningún servidor.");
    None
}
