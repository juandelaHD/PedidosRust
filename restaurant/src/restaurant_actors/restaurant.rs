use crate::messages::restaurant_messages::{SendToKitchen, ShareCommunicator};
use actix::prelude::*;
use common::logger::Logger;
use common::messages::{CancelOrder, NewOrder, UpdateOrderStatus, shared_messages::*};
use common::network::communicator::Communicator;
// use common::network::connections::{connect, connect_to_all};
// use common::network::peer_types::PeerType;
use common::types::dtos::{OrderDTO, UserDTO};
use common::types::order_status::OrderStatus;
use common::types::restaurant_info::RestaurantInfo;
use common::utils::random_bool_by_given_probability;
// use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
// use tokio::net::TcpStream;

use crate::restaurant_actors::delivery_assigner::DeliveryAssigner;
use crate::restaurant_actors::kitchen::Kitchen;
// use crate::kitchen::Kitchen;

pub struct Restaurant {
    /// Información básica del restaurante
    pub info: RestaurantInfo,
    /// Probabilidad de aceptar o rechazar un pedido.
    pub probability: f32,
    /// Canal de envío hacia la cocina.
    pub kitchen_sender: Addr<Kitchen>,
    pub delivery_assigner: Addr<DeliveryAssigner>,
    pub communicator: Arc<Communicator<Restaurant>>,
    pub my_socket_addr: SocketAddr,
    pub logger: Arc<Logger>,
}

impl Restaurant {
    pub fn new(
        info: RestaurantInfo,
        probability: f32,
        kitchen_sender: Addr<Kitchen>,
        delivery_assigner: Addr<DeliveryAssigner>,
        communicator: Arc<Communicator<Restaurant>>,
        my_socket_addr: SocketAddr,
        logger: Arc<Logger>,
    ) -> Self {
        Self {
            info,
            probability,
            kitchen_sender,
            delivery_assigner,
            communicator,
            my_socket_addr,
            logger,
        }
    }

    pub fn send_network_message(&self, message: NetworkMessage) {
        if let Some(sender) = self.communicator.sender.as_ref() {
            sender.do_send(message);
        } else {
            self.logger.error("Sender not initialized in communicator");
        }
    }
}

impl Actor for Restaurant {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        // No es necesario inicializar communicator aquí, ya viene inicializado por el constructor
        // Si querés enviar un mensaje inicial a la kitchen, podés hacerlo aquí
    }
}

impl Handler<StartRunning> for Restaurant {
    type Result = ();

    fn handle(&mut self, _msg: StartRunning, _ctx: &mut Self::Context) {
        self.logger.info("Starting restaurant...");
        self.send_network_message(NetworkMessage::WhoIsLeader(WhoIsLeader {
            origin_addr: self.my_socket_addr,
        }));
    }
}

impl Handler<NewLeaderConnection> for Restaurant {
    type Result = ();

    fn handle(&mut self, _msg: NewLeaderConnection, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info("Received NewLeaderConnection, but dynamic communicator switching is not supported en este modelo".to_string());
    }
}

impl Handler<LeaderIs> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: LeaderIs, _ctx: &mut Self::Context) -> Self::Result {
        self.logger.info(format!(
            "Received LeaderIs message with addr: {} (ignored, communicator is static)",
            msg.coord_addr
        ));
        self.send_network_message(NetworkMessage::RegisterUser(RegisterUser {
            origin_addr: self.my_socket_addr,
            user_id: self.info.id.clone(),
        }));
        self.logger.info(format!(
            "Sending msg register user with ID: {}",
            self.info.id
        ));
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
            NetworkMessage::RecoveredInfo(user_dto_opt) => {
                match user_dto_opt {
                    Some(user_dto) => match user_dto {
                        UserDTO::Restaurant(restaurant_dto) => {
                            if restaurant_dto.restaurant_id == self.info.id {
                                self.logger.info(format!(
                                    "Recovered info for Restaurant ID={}, updating local state...",
                                    restaurant_dto.restaurant_id
                                ));
                                // Si querés actualizar info, deberías hacerlo con interior mutability o reemplazar el struct

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
                    },
                    None => {
                        self.logger
                            .info("No recovered info found for this Delivery.");
                    }
                }
            }

            // Restaurant messages
            NetworkMessage::NewOrder(_msg_data) => ctx.address().do_send(_msg_data),

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

impl Handler<NewOrder> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: NewOrder, ctx: &mut Self::Context) -> Self::Result {
        self.logger
            .info(format!("Received NewOrder message: {:?}", msg));
        // Aquí podrías implementar la lógica para manejar un nuevo pedido
        // Por ejemplo, enviar un mensaje a la cocina o actualizar el estado del restaurante
        let mut new_order: OrderDTO = msg.order;
        match new_order.status {
            OrderStatus::Pending => {
                self.logger
                    .info(format!("Order pending with ID: {}", new_order.order_id));
                self.kitchen_sender.do_send(SendToKitchen { new_order });
            }
            OrderStatus::Authorized => {
                self.logger
                    .info(format!("Order authorized with ID: {}", new_order.order_id));
                if random_bool_by_given_probability(self.probability) {
                    // Simulamos que el restaurante acepta el pedido
                    self.logger.info(format!(
                        "Restaurant {} accepted order {}",
                        self.info.id, new_order.order_id
                    ));
                    new_order.status = OrderStatus::Pending;
                    self.kitchen_sender
                        .do_send(SendToKitchen { order: new_order });
                    self.send_network_message(NetworkMessage::UpdateOrderStatus(
                        UpdateOrderStatus { order: new_order },
                    ));
                } else {
                    // Simulamos que el restaurante rechaza el pedido
                    self.logger.info(format!(
                        "Restaurant {} rejected order {}",
                        self.info.id, new_order.order_id
                    ));
                    new_order.status =
                        OrderStatus::Cancelled("The restaurant rejected the order".to_string());
                    self.send_network_message(NetworkMessage::CancelOrder(CancelOrder {
                        order: new_order,
                    }));
                    // Aquí podrías enviar un mensaje de rechazo al coordinador o al cliente
                }
            }
            _ => {
                self.logger.warn(format!(
                    "Received NewOrder with non-pending nor authorized status: {:?}",
                    new_order
                ));
            }
        }
    }
}
