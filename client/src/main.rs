use common::constants::{SERVER_IP_ADDRESS, SERVER_PORT};
use common::logger::Logger;
use std::io::{self, Write};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> io::Result<()> {
    let main_logger = Arc::new(Logger::new("Main"));
    main_logger.info("Server is starting...");
    let server_addr = format!("{}:{}", SERVER_IP_ADDRESS, SERVER_PORT)
        .parse::<SocketAddr>()
        .expect("Dirección IP inválida");

    main_logger.info(format!(
        "Server address parsed successfully: {}",
        server_addr
    ));
    // Seguir con la inicialización...
    match TcpStream::connect(server_addr).await {
        Ok(_stream) => {
            println!("Conexión establecida con {}", server_addr);

            // // Enviar un mensaje al servidor
            // let msg = b"Hola desde el cliente!";
            // stream.write_all(msg).await?;
            // println!("Mensaje enviado: {:?}", String::from_utf8_lossy(msg));

            // // Leer la respuesta (si el servidor envía algo)
            // let mut buffer = vec![0; 1024];
            // let n = stream.read(&mut buffer).await?;
            // println!(
            //     "Respuesta del servidor: {}",
            //     String::from_utf8_lossy(&buffer[..n])
            // );
        }
        Err(e) => {
            eprintln!("Error al conectarse: {}", e);
        }
    }
    Ok(())
}

// // Intentamos la conexión
// match TcpStream::connect(server_addr).await {
//     Ok(mut stream) => {
//         println!("Conexión establecida con {}", server_addr);

//         // Enviar un mensaje al servidor
//         let msg = b"Hola desde el cliente!";
//         stream.write_all(msg).await?;
//         println!("Mensaje enviado: {:?}", String::from_utf8_lossy(msg));

//         // Leer la respuesta (si el servidor envía algo)
//         let mut buffer = vec![0; 1024];
//         let n = stream.read(&mut buffer).await?;
//         println!(
//             "Respuesta del servidor: {}",
//             String::from_utf8_lossy(&buffer[..n])
//         );
//     }
//     Err(e) => {
//         eprintln!("Error al conectarse: {}", e);
//     }
// }
