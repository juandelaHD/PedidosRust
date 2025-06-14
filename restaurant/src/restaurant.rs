use actix::prelude::*;
use common::logger::Logger;
use common::messages::shared_messages::*;
use common::network::communicator::Communicator;
use common::network::connections::{try_to_connect, connect_some};
use common::network::peer_types::PeerType;
use common::types::dtos::UserDTO;
use actix::fut::wrap_future;
use std::net::SocketAddr;
use tokio::net::TcpStream;

// use crate::kitchen::Kitchen;

pub struct Restaurant {
    /// Vector de direcciones de servidores
    pub servers: Vec<SocketAddr>,
    /// Identificador único del restaurante.
    pub restaurant_id: String,
    /// Posición actual del restaurante en coordenadas 2D.
    pub restaurant_position: (f32, f32),
    /// Probabilidad de aceptar o rechazar un pedido.
    pub probability: f32,
    /// Canal de envío hacia la cocina.
    //pub kitchen_sender: Addr<Kitchen>,
    /// Comunicador asociado al Server.
    // pub actual_communicator_addr: Option<SocketAddr>,
    pub communicator: Option<Communicator<Restaurant>>,
    pub pending_stream: Option<TcpStream>, // Guarda el stream hasta que arranque
    pub logger: Logger,
}

impl Restaurant {
    pub async fn new(
        servers: Vec<SocketAddr>,
        restaurant_id: String,
        restaurant_position: (f32, f32),
        probability: f32,
    ) -> Self {
        let logger = Logger::new(format!("Restaurant {}", &restaurant_id));
        
        logger.info(format!("Starting restaurant with ID: {}", restaurant_id));
        // Intentamos conectarnos a los servidores
        let pending_stream = connect_some(servers.clone(), PeerType::ClientType).await;

        if pending_stream.is_none() {
            panic!("Failed to connect to any server. Try again later.");
        }

        Self {
            servers,
            restaurant_id,
            restaurant_position,
            probability, // Puedes ajustar la probabilidad según tus necesidades
            communicator: None,
            pending_stream,
            logger,
        }
    }


    pub fn send_network_message(&self, message: NetworkMessage) {
        if let Some(communicator)  = &self.communicator {
            if let Some(sender) = &communicator.sender {
                sender.do_send(message);
            } else {
                self.logger.error("Sender not initialized in communicator");
            }
        } else {
            self.logger
                .error(&format!("Communicator not found!",));
        }
    } 

    pub fn start_running(&self, _ctx: &mut Context<Self>) {
        self.logger.info("Starting client...");
        let actual_socket_addr = self
            .communicator
            .as_ref()
            .map(|c| c.local_address)
            .expect("Socket address not initialized");
        self.send_network_message(NetworkMessage::WhoIsLeader(WhoIsLeader {
            origin_addr: actual_socket_addr,
            user_id: self.restaurant_id.clone(),
        }));
    }
}



impl Actor for Restaurant {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let communicator = Communicator::new( 
            self.pending_stream.take().expect("Pending stream should be set"),
            ctx.address(),
            PeerType::ClientType,
        );
        self.communicator = Some(communicator);

        self.start_running(ctx);
    }
}

pub struct UpdateCommunicator(pub Communicator<Restaurant>);

impl Message for UpdateCommunicator {
    type Result = ();
}

impl Handler<UpdateCommunicator> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: UpdateCommunicator, _ctx: &mut Self::Context) -> Self::Result {
        self.communicator = Some(msg.0);
    }
}

impl Handler<LeaderIs> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: LeaderIs, ctx: &mut Self::Context) -> Self::Result {
        let leader_addr = msg.coord_addr;
        let self_addr = ctx.address();
        let logger = self.logger.clone();
        let communicator_opt = self.communicator.as_ref().map(|c| c.peer_address);

        // Si ya estamos conectados al líder, no hacemos nada
        if Some(leader_addr) == communicator_opt {
            self.logger.info(format!(
                "Already connected to the leader at address: {}",
                leader_addr
            ));
            return;
        }
        // Si no estamos conectados al líder, intentamos conectarnos
        ctx.spawn(wrap_future(async move {            
            if let Some(new_stream) = try_to_connect(leader_addr).await {
                let new_communicator = Communicator::new(
                    new_stream,
                    self_addr.clone(),
                    PeerType::ClientType,
                );
                self_addr.do_send(UpdateCommunicator(new_communicator));
                logger.info(format!(
                    "Communicator updated with new peer address: {}",
                    leader_addr
                ));
            } else {
                logger.error(format!(
                    "Failed to connect to the new leader at {}",
                    leader_addr
                ));
            }
        }));
        let actual_socket_addr = self
            .communicator
            .as_ref()
            .map(|c| c.local_address)
            .expect("Socket address not set");
        self.send_network_message(NetworkMessage::WhoIsLeader(WhoIsLeader {
            origin_addr: actual_socket_addr,
            user_id: self.restaurant_id.clone(),
        }));
    }
}


impl Handler<NetworkMessage> for Restaurant {
    type Result = ();
    fn handle(&mut self, msg: NetworkMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            // All Users messages
            NetworkMessage::WhoIsLeader(_msg_data) => {
                self.logger
                    .error("Received a WhoIsLeader message, handle not implemented");
            }
            NetworkMessage::LeaderIs(msg_data) => ctx.address().do_send(msg_data),
            NetworkMessage::RegisterUser(_msg_data) => {
                self.logger
                    .info("Received RegisterUser message, not implemented yet");
            }
            NetworkMessage::RecoveredInfo(user_dto) => {
                match user_dto {
                    UserDTO::Restaurant(restaurant_dto) => {
                        if restaurant_dto.restaurant_id == self.restaurant_id {
                            self.logger.info(format!(
                                "Recovered info for Restaurant ID={}, updating local state...",
                                restaurant_dto.restaurant_id
                            ));

                            self.restaurant_position = restaurant_dto.restaurant_position;

                            ///////////////////////////////////////
                            //
                            //
                            // ACA FALTA RESETEAR LOS PEDIDOS PENDIENTES Y AUTORIZADOS
                            //
                            //////////////////////////////////////
                        } else {
                            self.logger.warn(format!(
                                "Received recovered info for a different delivery ({}), ignoring",
                                restaurant_dto.restaurant_id
                            ));
                        }
                    }
                    other => {
                        self.logger.warn(format!(
                            "Received recovered info of type {:?}, but I'm Delivery. Ignoring.",
                            other
                        ));
                    }
                }
            }

            // Restaurant messages
            NetworkMessage::UpdateOrderStatus(_msg_data) => {
                self.logger
                    .info("Received UpdateOrderStatus message, not implemented yet");
            }
            NetworkMessage::CancelOrder(_msg_data) => {
                self.logger
                    .info("Received CancelOrder message, not implemented yet");
            }
            NetworkMessage::OrderIsPreparing(_msg_data) => {
                self.logger
                    .info("Received OrderIsPreparing message, not implemented yet");
            }
            NetworkMessage::RequestDelivery(_msg_data) => {
                self.logger
                    .info("Received RequestDelivery message, not implemented yet");
            }
            NetworkMessage::DeliverThisOrder(_msg_data) => {
                self.logger
                    .info("Received DeliverThisOrder message, not implemented yet");
            }

            _ => {
                self.logger.info(format!(
                    "NetworkMessage descartado/no implementado: {:?}",
                    msg
                ));
            }
        }
    }
}

