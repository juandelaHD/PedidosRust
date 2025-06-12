use crate::client_actors::ui_handler::UIHandler;
use crate::messages::internal_messages::*;
use actix::prelude::*;
use common::logger::Logger;
use common::messages::NearbyRestaurants;
use common::messages::client_messages::*;
use common::messages::shared_messages::*;
use common::network::communicator::Communicator;
use common::network::connections::{connect, connect_to_all};
use common::network::peer_types::PeerType;
use common::types::dtos::ClientDTO;
use common::types::dtos::OrderDTO;
use common::types::order_status::OrderStatus;
use rand::Rng;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::TcpStream;

use common::types::dtos::UserDTO;

pub struct Client {
    /// Vector de direcciones de servidores
    pub servers: Vec<SocketAddr>,
    /// Identificador único del comensal.
    pub client_id: String,
    /// Posición actual del cliente en coordenadas 2D.
    pub client_position: (f32, f32),
    /// Restaurante elegido para el pedido.
    // pub selected_restaurant: Option<String>,
    /// Pedido actual.
    pub client_order: Option<OrderDTO>,
    /// Canal de envío hacia el actor `UIHandler`.
    pub ui_handler: Option<Addr<UIHandler>>,
    /// Comunicador asociado al `Server`.
    pub actual_communicator_addr: Option<SocketAddr>,
    pub communicators: Option<HashMap<SocketAddr, Communicator<Client>>>,
    pub pending_streams: HashMap<SocketAddr, TcpStream>, // Guarda el stream hasta que arranque
    pub my_socket_addr: SocketAddr,
    pub logger: Logger,
}

impl Client {
    pub async fn new(servers: Vec<SocketAddr>, id: String, client_position: (f32, f32)) -> Self {
        let pending_streams: HashMap<SocketAddr, TcpStream> = connect_to_all(servers.clone()).await;

        let logger = Logger::new(format!("Client {}", &id));

        if pending_streams.is_empty() {
            panic!("Unable to connect to any server.");
        }

        // Tomamos la primera conexión como referencia para my_socket_addr
        let my_socket_addr = pending_streams
            .values()
            .next()
            .expect("No stream available")
            .local_addr()
            .expect("Failed to get local socket addr");

        Self {
            servers,
            client_id: id,
            client_position,
            client_order: None,             // Inicializamos el pedido como None
            ui_handler: None, // Inicializamos el canal de envío hacia UIHandler como None
            actual_communicator_addr: None, // Podés usarlo para el líder actual
            communicators: Some(HashMap::new()), // Inicializamos el hashmap vacío para los communicators
            pending_streams: pending_streams,
            my_socket_addr,
            logger,
        }
    }

    pub fn send_network_message(&self, message: NetworkMessage) {
        if let (Some(communicators_map), Some(addr)) =
            (&self.communicators, self.actual_communicator_addr)
        {
            if let Some(communicator) = communicators_map.get(&addr) {
                if let Some(sender) = &communicator.sender {
                    sender.do_send(message);
                } else {
                    self.logger.error("Sender not initialized in communicator");
                }
            } else {
                self.logger
                    .error(&format!("Communicator not found for addr {}", addr));
            }
        } else {
            self.logger
                .error("Communicators map or actual_communicator_addr not initialized");
        }
    }
}

impl Actor for Client {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let mut communicators_map = HashMap::new();

        for (addr, stream) in self.pending_streams.drain() {
            let communicator = Communicator::new(stream, ctx.address(), PeerType::ClientType);
            communicators_map.insert(addr, communicator);
            self.actual_communicator_addr = Some(addr);
            self.logger
                .info(format!("Communicator started for {}", addr));
        }
        self.communicators = Some(communicators_map);
        let ui_handler = UIHandler::new(ctx.address(), self.logger.clone());
        self.ui_handler = Some(ui_handler.start());
    }
}

impl Handler<StartRunning> for Client {
    type Result = ();

    fn handle(&mut self, _msg: StartRunning, _ctx: &mut Self::Context) {
        self.logger.info("Starting client...");
        self.send_network_message(NetworkMessage::WhoIsLeader(WhoIsLeader {
            origin_addr: self.my_socket_addr,
            user_id: self.client_id.clone(),
        }));
    }
}

impl Handler<NewLeaderConnection> for Client {
    type Result = ();

    fn handle(&mut self, msg: NewLeaderConnection, ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Creating new Communicator for leader {}", msg.addr));

        let communicator = Communicator::new(msg.stream, ctx.address(), PeerType::ClientType);

        if let Some(communicators_map) = &mut self.communicators {
            communicators_map.insert(msg.addr, communicator);
            self.actual_communicator_addr = Some(msg.addr);
            self.logger.info(format!(
                "New leader Communicator established at {}",
                msg.addr
            ));
        } else {
            self.logger
                .error("Communicators map not initialized when creating new leader Communicator");
        }
    }
}

// RecoverProcedure
impl Handler<RecoverProcedure> for Client {
    type Result = ();

    fn handle(&mut self, msg: RecoverProcedure, _ctx: &mut Self::Context) -> Self::Result {
        match msg.user_info {
            UserDTO::Client(client_dto) => {
                if client_dto.client_id == self.client_id {
                    self.logger.info(format!(
                        "Recovering info for Client ID={}, updating local state...",
                        client_dto.client_id
                    ));
                    self.client_position = client_dto.client_position;
                    self.client_order = client_dto.client_order;

                    // Si tengo una orden activa, chequeo su estado
                    if let Some(client_dto) = &self.client_order {
                        self.logger.info(format!(
                            "Client ID={} tiene una orden activa con estado: {}",
                            client_dto.client_id, client_dto.status
                        ));

                        // Imprime los posibles estados del pedido
                        match client_dto.status {
                            OrderStatus::Cancelled => {
                                panic!("{}", client_dto.status)
                            }
                            _ => {
                                self.logger.info(format!("Estado: {}", client_dto.status));
                            }
                        }
                    } else {
                        // Si no tengo una orden activa, informo y solicito restaurantes cercanos
                        self.logger.info(format!(
                            "Client ID={} no tiene orden activa.",
                            self.client_id
                        ));
                        let new_client_dto = ClientDTO {
                            client_position: self.client_position,
                            client_id: self.client_id.clone(),
                            client_order: None, // No hay orden activa
                            time_stamp: std::time::SystemTime::now(),
                        };
                        self.send_network_message(NetworkMessage::RequestNearbyRestaurants(
                            RequestNearbyRestaurants {
                                client: new_client_dto.clone(),
                            },
                        ));
                    }
                } else {
                    self.logger.warn(format!(
                        "Received recovered info for a different client ({}), ignoring",
                        client_dto.client_id
                    ));
                }
            }
            other => {
                self.logger.warn(format!(
                    "Received recovered info of type {:?}, but I'm Client. Ignoring.",
                    other
                ));
            }
        }
    }
}

impl Handler<LeaderIs> for Client {
    type Result = ();

    fn handle(&mut self, msg: LeaderIs, ctx: &mut Self::Context) -> Self::Result {
        let leader_addr = msg.coord_addr;

        // Verificar si ya tenemos un Communicator para el nuevo líder
        if let Some(communicators_map) = &mut self.communicators {
            if communicators_map.contains_key(&leader_addr) {
                // Ya tenemos conexión con el líder, actualizar
                self.actual_communicator_addr = Some(leader_addr);
                self.logger.info(format!(
                    "Updated actual_communicator_addr to {}",
                    leader_addr
                ));
            } else {
                // No existe aún, tenemos que crearla (async dentro de sync handler usando spawn)
                let client_addr = ctx.address();
                let logger_clone = self.logger.clone();

                actix::spawn(async move {
                    logger_clone.info(format!("Connecting to new leader at {}", leader_addr));
                    if let Some(stream) = connect(leader_addr).await {
                        logger_clone.info(format!(
                            "Successfully connected to leader at {}",
                            leader_addr
                        ));

                        client_addr.do_send(NewLeaderConnection {
                            addr: leader_addr,
                            stream,
                        });
                    } else {
                        logger_clone.error(format!(
                            "Failed to connect to new leader at {}",
                            leader_addr
                        ));
                    }
                });
            }
            self.logger.info(format!(
                "Sending msg register user with ID: {}",
                self.client_id
            ));
            self.send_network_message(NetworkMessage::RegisterUser(RegisterUser {
                origin_addr: self.my_socket_addr,
                user_id: self.client_id.clone(),
            }));
        } else {
            self.logger.error("Communicators map not initialized");
        }
    }
}

impl Handler<SendThisOrder> for Client {
    type Result = ();

    fn handle(&mut self, msg: SendThisOrder, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Sending order to restaurant {}: {}",
            msg.selected_restaurant, msg.selected_dish
        ));

        let mut rng = rand::thread_rng();
        let order_id: u64 = rng.gen_range(1..=u64::MAX);

        let order = OrderDTO {
            order_id,
            client_id: self.client_id.clone(),
            restaurant_id: msg.selected_restaurant,
            dish_name: msg.selected_dish,
            status: OrderStatus::Pending, // Estado inicial del pedido
            delivery_id: None,            // No hay delivery asignado aún
            time_stamp: std::time::SystemTime::now(), // Marca de tiempo actual
            client_position: self.client_position, // Posición del cliente
        };

        // Enviar el pedido al servidor
        let network_message = NetworkMessage::RequestThisOrder(RequestThisOrder { order });
        self.send_network_message(network_message);
    }
}

impl Handler<NearbyRestaurants> for Client {
    type Result = ();

    fn handle(&mut self, msg: NearbyRestaurants, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Received NearbyRestaurants with {} restaurants",
            msg.restaurants.len()
        ));
        if let Some(ui_handler) = &self.ui_handler {
            ui_handler.do_send(SelectNearbyRestaurants {
                nearby_restaurants: msg.restaurants,
            });
        } else {
            self.logger.error("UIHandler not initialized");
        }
    }
}

impl Handler<NetworkMessage> for Client {
    type Result = ();
    fn handle(&mut self, msg: NetworkMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            // All Users messages
            NetworkMessage::LeaderIs(msg_data) => {
                self.logger.info(format!(
                    "Received LeaderIs message with addr: {}",
                    msg_data.coord_addr
                ));
                ctx.address().do_send(msg_data)
            }
            NetworkMessage::RecoveredInfo(user_dto) => match user_dto {
                UserDTO::Client(client_dto) => {
                    if client_dto.client_id == self.client_id {
                        self.logger.info(format!(
                            "Recovered info for Client ID={}, updating local state...",
                            client_dto.client_id
                        ));
                        ctx.address().do_send(RecoverProcedure {
                            user_info: UserDTO::Client(client_dto.clone()),
                        });
                    } else {
                        self.logger.warn(format!(
                            "Received recovered info for a different delivery ({}), ignoring",
                            client_dto.client_id
                        ));
                    }
                }
                other => {
                    self.logger.warn(format!(
                        "Received recovered info of type {:?}, but I'm Delivery. Ignoring.",
                        other
                    ));
                }
            },
            NetworkMessage::NoRecoveredInfo => {
                self.logger
                    .info("No recovered info received, proceeding with normal flow");
                // Aquí podrías enviar un mensaje para solicitar restaurantes cercanos
                let new_client_dto = ClientDTO {
                    client_position: self.client_position,
                    client_id: self.client_id.clone(),
                    client_order: None, // No hay orden activa
                    time_stamp: std::time::SystemTime::now(),
                };
                self.send_network_message(NetworkMessage::RequestNearbyRestaurants(
                    RequestNearbyRestaurants {
                        client: new_client_dto,
                    },
                ));
            }

            // Client messages
            NetworkMessage::NearbyRestaurants(msg_data) => {
                self.logger.info(format!(
                    "Received NearbyRestaurants message with {} restaurants",
                    msg_data.restaurants.len()
                ));
                ctx.address().do_send(msg_data);
            }
            NetworkMessage::AuthorizationResult(_msg_data) => {
                self.logger
                    .info("Received AuthorizationResult message, not implemented yet");
            }
            /*
            NetworkMessage::OrderFinalized(_msg_data) => {
                self.logger
                    .info("Received OrderFinalized message, not implemented yet");
            }
            */
            _ => {
                self.logger.info(format!(
                    "NetworkMessage descartado/no implementado: {:?}",
                    msg
                ));
            }
        }
    }
}
