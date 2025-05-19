use common::messages::client_messages::{
    NearbyRestaurantsList, OrderConfirmed, OrderRejected, RequestNearbyRestaurants,
    SubmitOrder,
};
use common::messages::shared_messages::{OrderStatusUpdate, WhoIsCoordinator};
use common::logger::Logger;
// use common::tcp_sender::TcpMessage;
use common::utils::get_rand_f32_tuple;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::time::timeout;
use uuid::Uuid;
use common::constants::TIMEOUT_SECONDS;

pub struct Client {
    servers: Vec<SocketAddr>,
    reader: Lines<BufReader<OwnedReadHalf>>,
    writer: OwnedWriteHalf,
    order_id: String,
    logger: Logger,
}

impl Client {
    pub async fn new(servers: Vec<SocketAddr>) -> Self {
        let mut input = String::new();

        println!("Please, introduce your name:");

        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                println!("Welcome to PedidosRust, {}!", input.trim());
            }
            Err(error) => {
                println!("Error reading name: {}", error);
            }
        }
        let logger = Logger::new(format!("CLIENT {}", input.trim()));
        logger.info("Trying to connect to coordinator...");
        
        let stream = connect_to_coordinator(servers.clone(), &logger)
            .await
            .expect("Could not connect to any coordinator.");
        let (rx, tx) = stream.into_split();

        logger.info("Successfully connected to coordinator.");

        Self {
            servers,
            reader: BufReader::new(rx).lines(),
            writer: tx,
            order_id: Uuid::new_v4().to_string(),
            logger,
        }
    }

    pub async fn run(&mut self) {
        let location = get_rand_f32_tuple();
        let request = RequestNearbyRestaurants { location };
        self.send_message(&request).await;

        let restaurant = self.await_restaurant_choice().await;
        self.logger
            .info(format!("Selected restaurant: {}", restaurant));

        let order = SubmitOrder {
            order_id: self.order_id.clone(),
            restaurant,
            items: vec!["Pizza".to_string(), "Coca".to_string()], // TODO: TALK ABOUT IMPLEMENTATION OF POSSIBLE ITEMS
        };
        self.send_message(&order).await;
        self.logger.info(format!("Submitting order {}", self.order_id));

        while let Ok(Some(line)) = self.reader.next_line().await {
            self.handle_message(line).await;
        }

        self.logger.warn("Server closed the connection.");
    }

    async fn await_restaurant_choice(&mut self) -> String {
        while let Ok(Some(line)) = self.reader.next_line().await {
            if let Ok(list) = serde_json::from_str::<NearbyRestaurantsList>(&line) {
                self.logger
                    .info(format!("Nearby restaurants: {:?}", list.restaurants));
                return list.restaurants.first().unwrap().clone();
            } else {
                self.logger.warn(format!("Ignored unrelated message: {}", line));
            }
        }
        panic!("[CLIENT] Did not receive any restaurant list."); // TODO: Handle this case properly
    }

    async fn handle_message(&mut self, line: String) {
        if let Ok(msg) = serde_json::from_str::<OrderConfirmed>(&line) {
            self.logger.info(format!("Order confirmed: {}", msg.order_id));
        } else if let Ok(msg) = serde_json::from_str::<OrderRejected>(&line) {
            self.logger.warn(format!(
                "Order rejected: {}, reason: {}",
                msg.order_id, msg.reason
            ));
        } else if let Ok(msg) = serde_json::from_str::<OrderStatusUpdate>(&line) {
            self.logger.info(format!(
                "Order status update: {}, status: {}",
                msg.order_id, msg.status
            ));
        } else {
            self.logger.warn(format!("Unknown message: {}", line));
        }
    }

    async fn send_message<T: serde::Serialize>(&mut self, msg: &T) {
        if let Ok(json) = serde_json::to_string(msg) {
            if let Err(e) = self
                .writer
                .write_all(format!("{}\n", json).as_bytes())
                .await
            {
                self.logger.error(format!("Error sending message: {}", e));
            }
        }
    }

}


async fn connect_to_coordinator(
    servers: Vec<SocketAddr>,
    logger: &Logger,
) -> Option<TcpStream> {
    for addr in servers {
        logger.info(format!("Trying server: {}", addr));
        if let Ok(mut stream) = TcpStream::connect(addr).await {
            let who_is_coord = WhoIsCoordinator;                // Serializo el mensaje en JSON + salto de línea para delimitar
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
                            return TcpStream::connect(coord_info).await.ok();
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