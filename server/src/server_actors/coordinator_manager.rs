use crate::messages::internal_messages::RegisterConnectionWithCoordinator;
use crate::server_actors::coordinator::Coordinator;
use actix::prelude::*;
use common::bimap::BiMap;
use common::logger::Logger;
use common::messages::coordinatormanager_messages::{CheckPongTimeout, LeaderElection, Ping, Pong};
use common::messages::shared_messages::NetworkMessage;
use common::messages::{LeaderIs, StartRunning, WhoIsLeader};
use common::network::communicator::Communicator;

use std::{
    collections::HashMap,
    net::SocketAddr,
    time::{Duration, Instant},
};

#[derive(Debug)]
pub struct CoordinatorManager {
    /// ID del CoordinatorManager.
    pub id: String,
    /// Direcciones de todos los nodos en el anillo.
    /// Lista ordenada de nodos actuales en el anillo.
    pub ring_nodes: Vec<SocketAddr>,
    /// Nodo coordinador actual.
    pub coordinator_actual: Option<SocketAddr>,
    /// Timestamps de los últimos heartbeats recibidos por nodo.
    pub heartbeat_timestamps: HashMap<SocketAddr, Instant>,
    pub coord_communicators: HashMap<SocketAddr, Communicator<Coordinator>>,
    pub coord_addresses: BiMap<SocketAddr, String>, // Mapa bidireccional de direcciones de coordinadores y sus IDs
    /// Dirección de este servidor.
    pub my_socket_addr: SocketAddr,
    /// Logger.
    pub logger: Logger,
    /// Dirección del actor `Coordinator` local.
    pub coordinator_addr: Addr<Coordinator>,
    pong_pending: bool,
    election_in_progress: bool,
}

impl Actor for CoordinatorManager {
    type Context = Context<Self>;
}

impl CoordinatorManager {
    pub fn new(
        id: String,
        my_coordinator_addr: SocketAddr,
        ring_nodes: Vec<SocketAddr>,
        coordinator_addr: Addr<Coordinator>,
    ) -> Self {
        Self {
            id,
            my_socket_addr: my_coordinator_addr,
            ring_nodes,
            coordinator_actual: None,
            heartbeat_timestamps: HashMap::new(),
            coord_communicators: HashMap::new(),
            coord_addresses: BiMap::new(),
            logger: Logger::new("COORDINATOR_MANAGER"),
            coordinator_addr,
            pong_pending: false,
            election_in_progress: false,
        }
    }

    pub fn start_leader_election(&mut self) {
        self.election_in_progress = true;
        let election = NetworkMessage::LeaderElection(LeaderElection {
            initiator: self.my_socket_addr,
            candidates: vec![self.my_socket_addr],
        });

        if let Some(next) = self.find_next_in_ring() {
            if let Err(err) = self.send_network_message(next, election) {
                self.logger
                    .error(format!("Failed to send message to {}: {}", next, err));
            }

            self.logger
                .info(format!("Iniciando elección. Enviado a {}", next));
        } else {
            self.logger.warn(
                "No hay siguiente nodo en el anillo para iniciar elección me autoproclamo líder",
            );
            // Si no hay siguiente nodo, me autoproclamo líder
            self.coordinator_actual = Some(self.my_socket_addr);
            self.election_in_progress = false;
            // Broadcast a todos los nodos que soy el líder
            self.broadcast_leader_is();
        }
    }

    fn find_next_in_ring(&self) -> Option<SocketAddr> {
        // Obtener claves de comunicadores conectados (sin incluirme)
        let mut nodes: Vec<_> = self
            .coord_communicators
            .keys()
            .copied()
            .filter(|addr| *addr != self.my_socket_addr)
            .collect();

        if nodes.is_empty() {
            return None;
        }

        // Incluirme para saber quién soy en la rotación
        nodes.push(self.my_socket_addr);
        nodes.sort();

        // Buscar el siguiente en el anillo
        let idx = nodes.iter().position(|x| *x == self.my_socket_addr)?;
        let next = nodes.get((idx + 1) % nodes.len())?;
        Some(*next)
    }

    fn start_heartbeat_checker(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(std::time::Duration::from_secs(5), |act, ctx| {
            if act.election_in_progress {
                act.logger
                    .info("Elección en progreso, omitiendo heartbeat.");
                return;
            }

            if let Some(leader) = act.coordinator_actual {
                if leader == act.my_socket_addr {
                    act.logger.info("Soy el líder, no hago ping.");
                    return;
                }

                if act.pong_pending {
                    act.logger
                        .warn("No se recibió Pong del líder. Iniciando elección...");
                    act.coordinator_actual = None;
                    act.election_in_progress = true;

                    // 👉 Remover el nodo que no responde
                    act.coord_communicators.remove(&leader);
                    act.coord_addresses.remove_by_key(&leader);
                    act.heartbeat_timestamps.remove(&leader);
                    act.ring_nodes.retain(|&n| n != leader); // limpieza opcional

                    act.start_leader_election();
                } else {
                    act.logger.info("Enviando Ping al líder...");

                    let local_addr = act
                        .coord_communicators
                        .get(&leader)
                        .map(|c| c.local_address)
                        .unwrap_or(act.my_socket_addr);

                    let result = act.send_network_message(
                        leader,
                        NetworkMessage::Ping(Ping { from: local_addr }),
                    );

                    match result {
                        Ok(_) => {
                            act.pong_pending = true;

                            let addr = ctx.address();
                            ctx.run_later(std::time::Duration::from_secs(3), move |_, _| {
                                addr.do_send(CheckPongTimeout);
                            });
                        }
                        Err(e) => {
                            act.logger.warn(format!(
                                "Fallo al enviar ping al líder: {}. Iniciando elección...",
                                e
                            ));
                            act.coordinator_actual = None;
                            act.election_in_progress = true;

                            // Limpieza también en error
                            act.coord_communicators.remove(&leader);
                            act.coord_addresses.remove_by_key(&leader);
                            act.heartbeat_timestamps.remove(&leader);
                            act.ring_nodes.retain(|&n| n != leader);

                            act.start_leader_election();
                        }
                    }
                }
            } else {
                act.logger
                    .info("No hay líder actual. Iniciando elección...");
                act.election_in_progress = true;
                act.start_leader_election();
            }
        });
    }

    fn send_network_message(
        &self,
        target: SocketAddr,
        message: NetworkMessage,
    ) -> Result<(), String> {
        if let Some(communicator) = self.coord_communicators.get(&target) {
            if let Some(sender) = &communicator.sender {
                self.logger
                    .info(format!("Sent message to {}: {:?}", target, message));
                sender.do_send(message);
                Ok(())
            } else {
                let err_msg = format!("Sender not initialized in communicator for {}", target);
                self.logger.error(&err_msg);
                Err(err_msg)
            }
        } else {
            let err_msg = format!("No communicator found for {}", target);
            self.logger.error(&err_msg);
            Err(err_msg)
        }
    }

    /// Envía un `NetworkMessage` a todos los nodos remotos conectados
    fn broadcast_network_message(&self, message: NetworkMessage) {
        for addr in self.coord_communicators.keys() {
            if *addr != self.my_socket_addr {
                if let Err(err) = self.send_network_message(*addr, message.clone()) {
                    self.logger
                        .error(format!("Failed to send message to {}: {}", *addr, err));
                }
            }
        }
    }

    fn broadcast_leader_is(&self) {
        if let Some(leader) = self.coordinator_actual {
            let message = NetworkMessage::LeaderIs(LeaderIs { coord_addr: leader });
            self.logger
                .info(format!("Broadcasting new leader: {}", leader));
            self.broadcast_network_message(message);
        }
    }

    fn broadcast_who_is_leader(&self) {
        for addr in self.coord_communicators.keys() {
            let communictor = self.coord_communicators.get(addr);
            if let Some(communicator) = communictor {
                let local_addr = communicator.local_address;
                let message = NetworkMessage::WhoIsLeader(WhoIsLeader {
                    origin_addr: local_addr,
                    user_id: self.id.clone(),
                });
                if *addr != self.my_socket_addr {
                    if let Err(err) = self.send_network_message(*addr, message.clone()) {
                        self.logger
                            .error(format!("Failed to send WhoIsLeader to {}: {}", *addr, err));
                    }
                }
                self.logger
                    .info(format!("Broadcasting WhoIsLeader to {}", addr));
            } else {
                self.logger
                    .warn(format!("No communicator found for {}", addr));
            }
        }
    }

    /// Preguntar a todos los nodos conocidos si hay un líder ya elegido
    fn ask_for_leader(&self, ctx: &mut Context<Self>) {
        self.broadcast_who_is_leader();

        // Esperamos X segundos para ver si alguien responde
        ctx.run_later(Duration::from_secs(1), |actor: &mut Self, _ctx| {
            if actor.coordinator_actual.is_none() {
                actor
                    .logger
                    .info("Asked all nodes for leader. No responses. Becoming leader...");
                actor.coordinator_actual = Some(actor.my_socket_addr);
                actor.coordinator_addr.do_send(LeaderIs {
                    coord_addr: actor.my_socket_addr,
                });
                actor.broadcast_leader_is();
            } else {
                actor.logger.info(format!(
                    "Leader response received before timeout: {:?}",
                    actor.coordinator_actual
                ));
            }
        });
    }

    fn handle_who_is_leader(&mut self, msg: WhoIsLeader, _ctx: &mut Context<Self>) {
        self.logger.info(format!(
            "Received WhoIsLeader from {} with ID={}, coordinator is: {:?}",
            msg.origin_addr, msg.user_id, self.coordinator_actual
        ));
        // Insertar la dirección del socket en el mapa de direcciones de coordinadores
        self.coord_addresses
            .insert(msg.origin_addr, msg.user_id.clone());

        if let Some(leader) = self.coordinator_actual {
            let response = NetworkMessage::LeaderIs(LeaderIs { coord_addr: leader });
            if let Some(registered_remote_addr) = self.coord_addresses.get_by_value(&msg.user_id) {
                if let Some(communicator) = self.coord_communicators.get(registered_remote_addr) {
                    if let Some(sender) = &communicator.sender {
                        sender.do_send(response);
                        self.logger
                            .info(format!("Sent LeaderIs to {}", msg.origin_addr));
                    } else {
                        self.logger.warn("Sender not initialized");
                    }
                } else {
                    self.logger
                        .warn(format!("No communicator to {}", msg.origin_addr));
                }
            } else {
                self.logger
                    .warn(format!("No origin address found for {}", msg.origin_addr));
            }
        } else {
            self.logger.info("No coordinator known yet to respond");
        }
    }

    fn handle_leader_is(&mut self, msg: LeaderIs, _ctx: &mut Context<Self>) {
        self.logger
            .info(format!("Received LeaderIs: {}", msg.coord_addr));
        self.election_in_progress = false;
        if self.coordinator_actual.is_none() {
            self.coordinator_actual = Some(msg.coord_addr);
            self.coordinator_addr.do_send(LeaderIs {
                coord_addr: msg.coord_addr,
            });
            self.logger
                .info(format!("Updated local coordinator to {}", msg.coord_addr));
        } else if self.coordinator_actual != Some(msg.coord_addr) {
            self.logger.warn(format!(
                "Pisé a mi coordinador porque llegó LeaderIs. Local: {:?}, Received: {}",
                self.coordinator_actual, msg.coord_addr
            ));
            self.coordinator_actual = Some(msg.coord_addr); //piso al actual
        }
    }
}

impl Handler<RegisterConnectionWithCoordinator> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, msg: RegisterConnectionWithCoordinator, _ctx: &mut Context<Self>) {
        // Registrar la conexión del CoordinatorManager
        self.coord_communicators
            .insert(msg.remote_addr, msg.communicator);
    }
}

impl Handler<StartRunning> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, _msg: StartRunning, ctx: &mut Context<Self>) {
        self.logger.info("Starting CoordinatorManager...");

        // Preguntar por el líder al inicio
        self.ask_for_leader(ctx);
        // Iniciar el chequeo de heartbeats al lider actual
        self.start_heartbeat_checker(ctx);
    }
}

impl Handler<WhoIsLeader> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, msg: WhoIsLeader, ctx: &mut Context<Self>) {
        // imprimir los comunicators que tenga
        self.handle_who_is_leader(msg, ctx);
    }
}

impl Handler<LeaderIs> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, msg: LeaderIs, _ctx: &mut Context<Self>) {
        self.logger
            .info(format!("Líder recibido: {}", msg.coord_addr));
        self.handle_leader_is(msg, _ctx);
    }
}

impl Handler<CheckPongTimeout> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, _msg: CheckPongTimeout, _ctx: &mut Self::Context) {
        if self.pong_pending {
            self.logger
                .warn("Timeout esperando Pong. Iniciando elección...");
            self.pong_pending = false;

            if let Some(dead_leader) = self.coordinator_actual {
                self.coord_communicators.remove(&dead_leader);
                self.coord_addresses.remove_by_key(&dead_leader);
                self.heartbeat_timestamps.remove(&dead_leader);
                self.ring_nodes.retain(|&n| n != dead_leader);
            }

            self.coordinator_actual = None;
            self.start_leader_election();
        }
    }
}

impl Handler<Ping> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, msg: Ping, _ctx: &mut Self::Context) {
        self.logger.info(format!("Recibido Ping de {}", msg.from));

        // Responder con Pong al remitente del Ping

        if let Err(err) = self.send_network_message(
            msg.from,
            NetworkMessage::Pong(Pong {
                from: self.my_socket_addr,
            }),
        ) {
            self.logger
                .error(format!("Failed to send message to {}: {}", msg.from, err));
        }
    }
}

impl Handler<Pong> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, msg: Pong, _ctx: &mut Self::Context) {
        self.logger.info(format!("Recibido Pong de {}", msg.from));
        // Pong recibido, ya no hay ping pendiente
        self.pong_pending = false;
    }
}

impl Handler<LeaderElection> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, msg: LeaderElection, _ctx: &mut Self::Context) {
        let mut candidates = msg.candidates.clone();

        if msg.initiator == self.my_socket_addr {
            // Completó el ciclo
            self.election_in_progress = false;
            let new_leader = *candidates.iter().min().unwrap();
            self.logger
                .info(format!("Elección terminada. Nuevo líder: {}", new_leader));
            self.coordinator_actual = Some(new_leader);

            // Broadcast a todos
            for addr in &self.ring_nodes {
                if let Err(err) = self.send_network_message(
                    *addr,
                    NetworkMessage::LeaderIs(LeaderIs {
                        coord_addr: new_leader,
                    }),
                ) {
                    self.logger
                        .error(format!("Failed to send LeaderIs to {}: {}", addr, err));
                }
            }
        } else {
            // Sumarme como candidato
            candidates.push(self.my_socket_addr);
            if let Some(next) = self.find_next_in_ring() {
                if let Err(err) = self.send_network_message(
                    next,
                    NetworkMessage::LeaderElection(LeaderElection {
                        initiator: msg.initiator,
                        candidates,
                    }),
                ) {
                    self.logger.error(format!(
                        "Failed to send LeaderElection to {}: {}",
                        next, err
                    ));
                }

                self.logger.info(format!("Pasando elección a {}", next));
            }
        }
    }
}
