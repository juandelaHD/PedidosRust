use crate::messages::internal_messages::RegisterConnectionWithCoordinator;
use crate::server_actors::coordinator::Coordinator;
use actix::prelude::*;
use common::bimap::BiMap;
use common::logger::Logger;
use common::messages::coordinatormanager_messages::LeaderElection;
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
        }
    }

    /// Cada cierto intervalo, revisa los heartbeats y detecta fallos.
    fn start_heartbeat_checker(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(Duration::from_secs(5), |actor, _ctx| {
            let now = Instant::now();
            let threshold = Duration::from_secs(10); // Si no hay heartbeat en 10s => fallo

            for node in &actor.ring_nodes {
                if let Some(last) = actor.heartbeat_timestamps.get(node) {
                    if now.duration_since(*last) > threshold {
                        actor.logger.warn(format!(
                            "Detected failure in node {}! Initiating leader election...",
                            node
                        ));
                        actor.start_leader_election();
                        break; // Solo iniciamos una elección a la vez
                    }
                } else {
                    actor.logger.warn(format!(
                        "No heartbeat received yet from {}. Skipping...",
                        node
                    ));
                }
            }
        });
    }

    fn start_leader_election(&self) {
        if let Some(next_node) = self.next_node_in_ring() {
            let election_msg = LeaderElection {
                candidates: vec![self.my_socket_addr],
            };

            match self
                .coordinator_addr
                .try_send(NetworkMessage::LeaderElection(election_msg))
            {
                Ok(()) => {
                    self.logger
                        .info(format!("Sent LeaderElection to {}", next_node));
                }
                Err(e) => {
                    self.logger.error(format!(
                        "Could not send LeaderElection to next node: {:?}",
                        e
                    ));
                }
            }
        } else {
            self.logger
                .error("No next node found in ring. Ring might be incomplete.");
        }
    }

    /// Obtiene el siguiente nodo en el anillo
    fn next_node_in_ring(&self) -> Option<SocketAddr> {
        if let Some(pos) = self
            .ring_nodes
            .iter()
            .position(|&n| n == self.my_socket_addr)
        {
            let next_idx = (pos + 1) % self.ring_nodes.len();
            Some(self.ring_nodes[next_idx])
        } else {
            None
        }
    }

    /// Enviar un mensaje de red a un nodo específico
    // fn send_network_message(&self, target: SocketAddr, message: NetworkMessage) {
    //     // Aquí deberías tener un mapa o lógica para obtener el Addr/actor del nodo target.
    //     // Como ejemplo, enviamos a `coordinator_addr` si coincide con target.
    //     if target == self.my_addr {
    //         // Si el target es el mismo nodo local, enviamos al coordinator local
    //         self.coordinator_addr.do_send(message);
    //     } else {
    //         // Aquí pondrías la lógica para enviar a otro nodo (ej: vía TCP o algún actor remoto)
    //         self.logger.warn(format!(
    //             "Sending to external node {} no implementado aún",
    //             target
    //         ));
    //     }
    // }

    fn send_network_message(&self, target: SocketAddr, message: NetworkMessage) {
        if let Some(communicator) = self.coord_communicators.get(&target) {
            if let Some(sender) = &communicator.sender {
                self.logger
                    .info(format!("Sent message to {}: {:?}", target, message));
                sender.do_send(message);
            } else {
                self.logger.error(format!(
                    "Sender not initialized in communicator for {}",
                    target
                ));
            }
        } else {
            self.logger
                .error(format!("No communicator found for {}", target));
        }
    }

    /// Envía un `NetworkMessage` a todos los nodos remotos conectados
    fn broadcast_network_message(&self, message: NetworkMessage) {
        for addr in self.coord_communicators.keys() {
            if *addr != self.my_socket_addr {
                self.send_network_message(*addr, message.clone());
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
                    self.send_network_message(*addr, message.clone());
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

        if self.coordinator_actual.is_none() {
            self.coordinator_actual = Some(msg.coord_addr);
            self.coordinator_addr.do_send(LeaderIs {
                coord_addr: msg.coord_addr,
            });
            self.logger
                .info(format!("Updated local coordinator to {}", msg.coord_addr));
        } else if self.coordinator_actual != Some(msg.coord_addr) {
            self.logger.warn(format!(
                "Conflicting LeaderIs received. Local: {:?}, Received: {}",
                self.coordinator_actual, msg.coord_addr
            ));
            // Aquí podrías iniciar una nueva elección si sospechas inconsistencias
        }
    }

    fn handle_leader_election(&mut self, msg: LeaderElection, _ctx: &mut Context<Self>) {
        let mut candidates = msg.candidates;

        if !candidates.contains(&self.my_socket_addr) {
            candidates.push(self.my_socket_addr);
        }

        if candidates.first() == Some(&self.my_socket_addr) {
            // Soy el iniciador y el mensaje dio la vuelta
            if let Some(new_leader) = candidates.iter().min() {
                self.coordinator_actual = Some(*new_leader);
                self.logger
                    .info(format!("New leader elected: {}", new_leader));
                self.broadcast_leader_is();
            }
        } else {
            // Reenviar al siguiente nodo en el anillo con la lista actualizada de candidatos
            if let Some(next_node) = self.next_node_in_ring() {
                self.send_network_message(
                    next_node,
                    NetworkMessage::LeaderElection(LeaderElection { candidates }),
                );
            } else {
                self.logger
                    .warn("No next node found to forward LeaderElection");
            }
        }
    }
}

// impl Handler<LeaderElection> for CoordinatorManager {
//     type Result = ();

//     fn handle(&mut self, msg: LeaderElection, ctx: &mut Context<Self>) -> Self::Result {
//         let mut candidates = msg.candidates;

//         // Si este nodo no está en la lista de candidatos, se agrega
//         if !candidates.contains(&self.my_addr) {
//             candidates.push(self.my_addr);
//         }

//         if candidates[0] == self.my_addr {
//             // El mensaje dio la vuelta al nodo iniciador, elegir líder
//             if let Some(new_leader) = candidates.iter().min() {
//                 self.coordinator = Some(*new_leader);
//                 self.logger
//                     .info(format!("New leader elected: {}", new_leader));
//                 self.broadcast_leader(ctx);
//             }
//         } else {
//             // Reenviar al siguiente nodo en el anillo con la lista actualizada de candidatos
//             if let Some(next_node) = self.next_node_in_ring() {
//                 self.send_network_message(
//                     next_node,
//                     NetworkMessage::LeaderElection(LeaderElection { candidates }),
//                 );
//             }
//         }
//     }
// }

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

        // Iniciar el chequeo de heartbeats
        // self.start_heartbeat_checker(ctx);

        // Preguntar por el líder al inicio
        self.ask_for_leader(ctx);
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
        self.handle_leader_is(msg, _ctx);
    }
}
