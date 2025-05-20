use actix::prelude::*;
use common::constants::TIMEOUT_SECONDS;
use common::logger::Logger;
use common::messages::payment_messages::*;
use common::network::socket_writer::{TCPMessage, SocketWriter};
use common::network::socket_reader::{SocketReader, IncomingLine};
use common::utils::random_bool_by_probability;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};

use std::time::Duration;

pub struct PaymentGatewayActor {
    socket_writer: Addr<SocketWriter>,
    socket_reader: Addr<SocketReader<PaymentGatewayActor>>,
    logger: Logger,
}

impl Actor for PaymentGatewayActor {
    type Context = Context<Self>;
}

impl PaymentGatewayActor {
    pub fn new(stream: TcpStream, addr: SocketAddr) -> Addr<Self> {
        PaymentGatewayActor::create(|ctx| {
            let logger = Logger::new(format!("PAYMENT_GATEWAY [{}]", addr));

            logger.info(format!("Payment gateway actor started for [{}]", addr));
            let (read_half, write_half) = tokio::io::split(stream);

            // Lector de mensajes
            let socket_reader = SocketReader::new(read_half, ctx.address()).start();

            // Escritor de mensajes (actor)
            let socket_writer = SocketWriter::new(write_half).start();

            PaymentGatewayActor {
                socket_writer,
                socket_reader,
                logger,
            }
        })
    }

    pub async fn start(addr: SocketAddr) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(addr).await?;
        println!("[{}] Listening for payment connections", addr);

        loop {
            match listener.accept().await {
                Ok((stream, client_addr)) => {
                    println!("[{}] Connection received from {:?}", addr, client_addr);
                    Self::new(stream, client_addr);
                }
                Err(e) => {
                    eprintln!("[{}] Failed to accept connection: {:?}", addr, e);
                    tokio::time::sleep(Duration::from_secs(TIMEOUT_SECONDS)).await;
                }
            }
        }
    }
}

impl Handler<IncomingLine> for PaymentGatewayActor {
    type Result = ();
    fn handle(&mut self, msg: IncomingLine, ctx: &mut Self::Context) {
        let socket_writer = self.socket_writer.clone();
        let logger = self.logger.clone();
        let raw = msg.0;

        match serde_json::from_str::<PaymentGatewayRequest>(&raw) {
            Ok(PaymentGatewayRequest::CheckAuth(auth_msg)) => {
                let authorized = random_bool_by_probability();

                logger.info(&format!(
                    "Received payment authorization request for client [{}]",
                    auth_msg.client_id
                ));

                logger.info(&format!(
                    "Payment [{}] for client [{}]",
                    if authorized { "AUTHORIZED" } else { "REJECTED" },
                    auth_msg.client_id
                ));

                let response = PaymentAuthorizationResult {
                    client_id: auth_msg.client_id,
                    authorized,
                };

                match serde_json::to_string(&response) {
                    Ok(json) => socket_writer.do_send(TCPMessage(json)),
                    Err(_) => logger.error("Failed to serialize AuthorizationResponse"),
                }
            }

            Ok(PaymentGatewayRequest::Execute(payment_msg)) => {
                logger.info(&format!(
                    "Executing payment for passenger [{}]",
                    payment_msg.client_id
                ));

                let response = PaymentGatewayResponse::Success;

                match serde_json::to_string(&response) {
                    Ok(json) => socket_writer.do_send(TCPMessage(json)),
                    Err(_) => logger.error("Failed to serialize PaymentDone"),
                }
            }

            Err(_) => {
                logger.error("Failed to deserialize incoming message");
                ctx.stop(); // stop actor if data is invalid (optional)
            }
        }
    }
}
