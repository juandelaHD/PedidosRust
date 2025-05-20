use actix::fut::stream;
use actix::prelude::*;
use std::net::SocketAddr;
use serde::Serialize;
use uuid::Uuid;
use common::logger::Logger;
use common::utils::get_rand_f32_tuple;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::timeout;
use common::messages::shared_messages::{StartRunning, Reconnect, OrderStatusUpdate, WhoIsCoordinator};
use common::network::socket_reader::{IncomingLine, SocketReader};
use common::network::socket_writer::{TCPMessage, SocketWriter};
use common::messages::client_messages::*;
use common::constants::TIMEOUT_SECONDS;
use std::time::Duration;
use std::sync::Arc;
use tokio::net::TcpStream;

pub struct Client {
    stream: Option<TcpStream>,
    servers: Vec<SocketAddr>,
    socket_writer: Option<Arc<Addr<SocketWriter>>>,
    order_id: String,
    logger: Logger,
    order_submitted: bool,
}

impl Client {
    pub async fn new(servers: Vec<SocketAddr>) -> Self {
        let order_id = Uuid::new_v4().to_string();
        let logger = Logger::new(format!("CLIENT {}", &order_id[..8]));
        
        let tcp_stream = connect_please(servers.clone(), &logger).await;
        if let Some(stream) = tcp_stream {
            Client {
                stream: Some(stream),
                servers,
                socket_writer: None,
                order_id,
                logger,
                order_submitted: false,
            }
        } else {
            panic!("Unable to connect to any server.");
        }
    }
   
    fn send_message<T: Serialize>(&self, msg: &T) {
        if let Ok(json) = serde_json::to_string(msg) {
            if let Some(writer) = &self.socket_writer {
                writer.do_send(TCPMessage(json));
            } else {
                self.logger.warn("No socket_writer available");
            }
        } else {
            self.logger.warn("Failed to serialize message");
        }
    }

}

impl Actor for Client {
    type Context = Context<Self>;
}

#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct UpdateSocketWriter(pub Addr<SocketWriter>);

impl Handler<UpdateSocketWriter> for Client {
    type Result = ();

    fn handle(&mut self, msg: UpdateSocketWriter, _ctx: &mut Context<Self>) {
        self.socket_writer = Some(Arc::new(msg.0));
        self.logger.info("Socket writer updated");
    }
}

impl Handler<Reconnect> for Client {
    type Result = ();

    fn handle(&mut self, _msg: Reconnect, ctx: &mut Context<Self>) {
        let servers = self.servers.clone();
        let addr = ctx.address();
        let logger = self.logger.clone();

        ctx.spawn(actix::fut::wrap_future(async move {
            connect_logic(servers, addr, logger).await;
        }));
    }
}

impl Handler<StartRunning> for Client {
    type Result = ();

    fn handle(&mut self, _msg: StartRunning, ctx: &mut Self::Context) {
        if let Some(stream) = self.stream.take() { 
            let (read_half, write_half) = stream.into_split();
            let socket_writer = SocketWriter::new(write_half).start();
            self.socket_writer = Some(Arc::new(socket_writer));
            self.logger.info("Client started");
            let _socket_reader = SocketReader::new(read_half, ctx.address()).start();
            self.logger.info("Socket reader started");
            
            // TODO: Handle situation where the user restarts the client and the server is already running
            // In that case, we should not send the StartRunning message again, just return our status where we left off
            // Maybe we should send a "RecoverState" message to the server and wait for a response

            let (lat, lon) = get_rand_f32_tuple();
            let nearby_restaurants = RequestNearbyRestaurants {
                location: (lat, lon),
            };
            self.send_message(&nearby_restaurants);
            self.logger.info(format!("Requesting nearby restaurants at ({}, {})", lat, lon));
        } else {
            self.logger.error("No stream available");
        }

    }
}

impl Handler<IncomingLine> for Client {
    type Result = ();

    fn handle(&mut self, msg: IncomingLine, _ctx: &mut Self::Context) {
        let line = msg.0;

        if !self.order_submitted {
            if let Ok(list) = serde_json::from_str::<NearbyRestaurantsList>(&line) {
                self.logger.info(format!("Nearby restaurants: {:?}", list.restaurants));
                if let Some(restaurant) = list.restaurants.first() {
                    let order = SubmitOrder {
                        order_id: self.order_id.clone(),
                        restaurant: restaurant.clone(),
                        items: vec!["Pizza".to_string(), "Coca".to_string()],
                    };
                    self.send_message(&order);
                    self.logger.info(format!("Submitting order {}", self.order_id));
                    self.order_submitted = true;
                }
                return;
            }
        }

        if let Ok(msg) = serde_json::from_str::<OrderConfirmed>(&line) {
            self.logger.info(format!("Order confirmed: {}", msg.order_id));
        } else if let Ok(msg) = serde_json::from_str::<OrderRejected>(&line) {
            self.logger.warn(format!("Order rejected: {}, reason: {}", msg.order_id, msg.reason));
        } else if let Ok(msg) = serde_json::from_str::<OrderStatusUpdate>(&line) {
            self.logger.info(format!("Order status update: {}, status: {}", msg.order_id, msg.status));
        } else {
            self.logger.warn(format!("Unknown message: {}", line));
        }

        // TODO: Every time we handle a message, we should send the "PersistState" message
        // to the server to persist our state
    }
}


pub async fn connect_to_coordinator(
    servers: Vec<SocketAddr>,
    logger: &Logger,
) -> Option<tokio::net::TcpStream> {
    for addr in servers {
        logger.info(format!("Trying server: {}", addr));
        if let Ok(mut stream) = tokio::net::TcpStream::connect(addr).await {
            let who_is_coord = WhoIsCoordinator;
            if let Ok(msg_json) = serde_json::to_string(&who_is_coord) {
                if stream.write_all(format!("{}\n", msg_json).as_bytes()).await.is_ok() {
                    let mut reader = BufReader::new(&mut stream);
                    let mut line = String::new();

                    if timeout(Duration::from_secs(TIMEOUT_SECONDS), reader.read_line(&mut line))
                        .await
                        .is_ok()
                    {
                        if let Ok(coord_info) = serde_json::from_str::<SocketAddr>(line.trim()) {
                            logger.info(format!("Coordinator is at {}", coord_info));
                            return tokio::net::TcpStream::connect(coord_info).await.ok();
                        } else {
                            logger.warn(format!("Failed to parse coordinator address: {}", line));
                        }
                    } else {
                        logger.warn("Timeout waiting for coordinator response");
                    }
                } else {
                    logger.warn("Failed to send WhoIsCoordinator message");
                }
            }
        }
    }
    logger.error("Could not connect to any coordinator.");
    None
}


async fn connect_logic(
    servers: Vec<SocketAddr>,
    addr: Addr<Client>,
    logger: Logger,
) {
    loop {
        logger.info("Trying to connect to a coordinator...");
        if let Some(stream) = connect_to_coordinator(servers.clone(), &logger).await {
            logger.info("Connected to coordinator");

            let (read_half, write_half) = stream.into_split();
            let socket_writer = SocketWriter::new(write_half).start();

            addr.do_send(UpdateSocketWriter(socket_writer.clone()));
            let _reader = SocketReader::new(read_half, addr.clone()).start();

            // TODO: Handle situation where the user restarts the client and the server is already running
            // In that case, we should not send the StartRunning message again, just return our status where we left off
            // Maybe we should send a "RecoverState" message to the server and wait for a response
            addr.do_send(StartRunning);
            return;
        } else {
            logger.error("Failed to connect. Retrying in 2 seconds...");
            tokio::time::sleep(Duration::from_secs(TIMEOUT_SECONDS)).await;
        }
    }
}

async fn connect_please(
    servers: Vec<SocketAddr>,
    logger: &Logger,
) -> Option<TcpStream> {
    for addr in servers {
        logger.info(format!("Trying server: {}", addr));
        if let Ok(stream) = TcpStream::connect(addr).await {
            return Some(stream);
        }
    }
    None
}