use std::{collections::HashMap, net::SocketAddr, time::{Instant, Duration}};


use actix::prelude::*;
use common::logger::Logger;
use common::messages::shared_messages::{NetworkMessage};
use common::messages::coordinatormanager_messages::LeaderElection;


use crate::server_actors::server_actor::Coordinator;
use std::sync::Arc;




#[derive(Debug)]
pub struct CoordinatorManager {
    /// Lista ordenada de nodos en el anillo.
    pub ring_nodes: Vec<SocketAddr>,
    /// Nodo coordinador actual.
    pub coordinator: Option<SocketAddr>,
    /// Timestamps de los últimos heartbeats recibidos por nodo.
    pub heartbeat_timestamps: HashMap<SocketAddr, Instant>,


    /// Dirección de este servidor.
    pub my_addr: SocketAddr,
    /// Logger.
    pub logger: Arc<Logger>,
    /// Dirección del actor `Coordinator` local.
    pub coordinator_addr: Addr<Coordinator>,
}

impl Actor for CoordinatorManager {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.logger.info("CoordinatorManager started.");
        // Arrancamos el chequeo periódico de heartbeats
        //self.start_heartbeat_checker(ctx); ///////////////////////////////////////////// ENCENDER
        // Preguntamos por líder al arrancar
        self.ask_for_leader(ctx);
    }
}

impl CoordinatorManager {
    pub fn new(
        my_addr: SocketAddr,
        ring_nodes: Vec<SocketAddr>,
        coordinator_addr: Addr<Coordinator>,
        logger: Arc<Logger>,
    ) -> Addr<Self> {
        CoordinatorManager::create(move |_ctx| CoordinatorManager {
            my_addr,
            ring_nodes,
            coordinator: None,
            heartbeat_timestamps: HashMap::new(),
            coordinator_addr,
            logger,
        })
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
                candidates: vec![self.my_addr],
            };

            match self.coordinator_addr.try_send(NetworkMessage::LeaderElection(election_msg)) {
                Ok(()) => {
                    self.logger.info(format!("Sent LeaderElection to {}", next_node));
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
        if let Some(pos) = self.ring_nodes.iter().position(|&n| n == self.my_addr) {
            let next_idx = (pos + 1) % self.ring_nodes.len();
            Some(self.ring_nodes[next_idx])
        } else {
            None
        }
    }

    /// Enviar un mensaje de red a un nodo específico
    fn send_network_message(&self, target: SocketAddr, message: NetworkMessage) {
        // Aquí deberías tener un mapa o lógica para obtener el Addr/actor del nodo target.
        // Como ejemplo, enviamos a `coordinator_addr` si coincide con target.
        if target == self.my_addr {
            // Si el target es el mismo nodo local, enviamos al coordinator local
            self.coordinator_addr.do_send(message);
        } else {
            // Aquí pondrías la lógica para enviar a otro nodo (ej: vía TCP o algún actor remoto)
            self.logger.warn(format!("Sending to external node {} no implementado aún", target));
        }
    }

    /// Preguntar a todos los nodos conocidos si hay un líder ya elegido
    fn ask_for_leader(&self, ctx: &mut Context<Self>) {
        for node in &self.ring_nodes {
            if *node != self.my_addr {
                self.send_network_message(*node, NetworkMessage::LeaderElection(LeaderElection { candidates: vec![] }));
            }
        }
        // Si no recibimos respuesta en X segundos, podemos autoproclamarnos líderes (pendiente de implementar)
    }

    /// Cuando se elige un nuevo líder, informar a todos los nodos
    fn broadcast_leader(&self, ctx: &mut Context<Self>) {
        if let Some(leader) = self.coordinator {
            for node in &self.ring_nodes {
                self.send_network_message(*node, NetworkMessage::LeaderElection(LeaderElection {
                    candidates: vec![leader],
                }));
            }
        }
    }
}

impl Handler<LeaderElection> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, msg: LeaderElection, ctx: &mut Context<Self>) -> Self::Result {
        let mut candidates = msg.candidates;

        // Si este nodo no está en la lista de candidatos, se agrega
        if !candidates.contains(&self.my_addr) {
            candidates.push(self.my_addr);
        }

        if candidates[0] == self.my_addr {
            // El mensaje dio la vuelta al nodo iniciador, elegir líder
            if let Some(new_leader) = candidates.iter().min() {
                self.coordinator = Some(*new_leader);
                self.logger.info(format!("New leader elected: {}", new_leader));
                self.broadcast_leader(ctx);
            }
        } else {
            // Reenviar al siguiente nodo en el anillo con la lista actualizada de candidatos
            if let Some(next_node) = self.next_node_in_ring() {
                self.send_network_message(next_node, NetworkMessage::LeaderElection(LeaderElection { candidates }));
            }
        }
    }
}