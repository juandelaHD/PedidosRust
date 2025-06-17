use crate::messages::internal_messages::{
    GetAllStorage, GetLogsFromIndex, GetMinLogIndex, RegisterConnectionWithCoordinator,
};
use crate::server_actors::coordinator::Coordinator;
use crate::server_actors::storage::Storage;
use actix::prelude::*;
use colored::Color;
use common::bimap::BiMap;
use common::constants::{
    INTERVAL_HEARTBEAT, INTERVAL_STORAGE, TIMEOUT_HEARTBEAT, TIMEOUT_LEADER_RESPONSE,
};
use common::logger::Logger;
use common::messages::coordinatormanager_messages::{CheckPongTimeout, LeaderElection, Ping, Pong};
use common::messages::shared_messages::{ConnectionClosed, NetworkMessage};
use common::messages::{
    ApplyStorageUpdates, LeaderIdIs, RequestAllStorage, RequestNewStorageUpdates, StartRunning,
    StorageSnapshot, StorageUpdates, WhoIsLeader,
};
use common::network::communicator::Communicator;
use common::types::dtos::Snapshot;
use std::{collections::HashMap, net::SocketAddr};

#[derive(Debug)]
pub struct CoordinatorManager {
    /// ID del CoordinatorManager.
    pub id: String,
    /// Direcciones de todos los nodos en el anillo.
    pub ring_nodes: HashMap<String, SocketAddr>,
    /// Nodo coordinador actual.
    pub coordinator_actual: Option<SocketAddr>,
    /// Mapa de direcciones de nodos coordinadores y sus comunicadores.
    pub coord_communicators: HashMap<SocketAddr, Communicator<Coordinator>>,
    /// Mapa bidireccional de direcciones de coordinadores y sus IDs
    pub coord_addresses: BiMap<SocketAddr, String>,
    /// Dirección de este servidor.
    pub my_socket_addr: SocketAddr,
    /// Logger.
    /// Timestamps de los últimos heartbeats recibidos por nodo.
    // pub heartbeat_timestamps: HashMap<SocketAddr, Instant>,
    pub logger: Logger,
    /// Dirección del actor `Coordinator` local.
    pub coordinator_addr: Addr<Coordinator>,
    /// Indica si hay un Pong pendiente del líder.
    pong_pending: bool,
    /// Indica si hay una elección de líder en progreso.
    election_in_progress: bool,
    /// Dirección del Storage
    pub storage: Addr<Storage>,
    pong_leader_addr: Option<SocketAddr>,
    waiting_pong_timer: Option<actix::SpawnHandle>,
    get_storage_updates_timer: Option<actix::SpawnHandle>,
}

impl Actor for CoordinatorManager {
    type Context = Context<Self>;
}

impl CoordinatorManager {
    pub fn new(
        id: String,
        my_coordinator_addr: SocketAddr,
        ring_nodes: HashMap<String, SocketAddr>,
        coordinator_addr: Addr<Coordinator>,
        storage: Addr<Storage>,
    ) -> Self {
        let mut coord_addresses = BiMap::new();
        for (id, addr) in ring_nodes.iter() {
            coord_addresses.insert(addr.clone(), id.clone());
        }

        Self {
            id,
            my_socket_addr: my_coordinator_addr,
            ring_nodes,
            coordinator_actual: None,
            coord_communicators: HashMap::new(),
            coord_addresses,
            //heartbeat_timestamps: HashMap::new(),
            logger: Logger::new("COORDINATOR_MANAGER", Color::BrightCyan),
            coordinator_addr,
            pong_pending: false,
            election_in_progress: false,
            storage,
            pong_leader_addr: None,
            waiting_pong_timer: None,
            get_storage_updates_timer: None,
        }
    }

    pub fn start_leader_election(&mut self) {
        self.election_in_progress = true;
        let election = NetworkMessage::LeaderElection(LeaderElection {
            initiator: self.id.clone(),
            candidates: vec![self.id.clone()],
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

    pub fn start_storage_updates_checker(&mut self, ctx: &mut Context<Self>) {
        if self.get_storage_updates_timer.is_some() {
            return; // Ya está corriendo
        }
        if self.election_in_progress {
            self.logger.info("Elección en progreso, omitiendo Updates.");
            return;
        }

        // let addr = ctx.address();
        let storage_addr = self.storage.clone();

        let handler = ctx.run_interval(INTERVAL_STORAGE, move |act, ctx| {
            if act.election_in_progress {
                act.logger
                    .info("Elección en progreso, omitiendo actualizaciones de Storage.");
                return;
            }


            // ¡Calcular el nodo anterior en cada tick!
            let previous_node_addr_opt = act.find_previous_in_ring();
            if previous_node_addr_opt.is_none() {
                act.logger.warn("No previous node found for storage updates.");
                return;
            }
            let previous_node_addr = previous_node_addr_opt.unwrap();

            // Enviar GetMinLogIndex al storage y esperar la respuesta
            storage_addr
                .send(GetMinLogIndex)
                .into_actor(act)
                .then(move |res, act, _ctx| {
                    match res {
                        Ok(min_log_index) => {
                            act.logger.info(format!(
                                "Recibido minLogIndex: {:?}, enviando RequestNewStorageUpdates al siguiente nodo.",
                                min_log_index
                            ));
                            if let Err(e) = act.send_network_message(previous_node_addr, NetworkMessage::RequestNewStorageUpdates(RequestNewStorageUpdates {
                                coordinator_id: act.id.clone(),
                                start_index: min_log_index,
                            })) {
                                act.logger.warn(format!(
                                    "Error al enviar RequestNewStorageUpdates: {}",
                                    e
                                ));
                            }
                        }
                        Err(e) => {
                            act.logger.warn(format!(
                                "Error al obtener minLogIndex de storage: {:?}",
                                e
                            ));
                        }
                    }
                    fut::ready(())
                })
                .spawn(ctx);
        });

        self.get_storage_updates_timer = Some(handler);
    }

    fn find_next_in_ring(&self) -> Option<SocketAddr> {
        // Obtener claves de comunicadores conectados (sin incluirme)
        let mut nodes_ids = self.ring_nodes.keys().cloned().collect::<Vec<_>>();
        nodes_ids.sort();

        let nodes = nodes_ids
            .iter()
            .filter_map(|id| {
                self.coord_addresses
                    .get_by_value(id)
                    .cloned()
                    .and_then(|addr| {
                        if self.coord_communicators.contains_key(&addr) || id == &self.id {
                            Some(addr)
                        } else {
                            self.logger.warn(format!(
                                "Address {} for ID {} not in coord_communicators",
                                addr, id
                            ));
                            None
                        }
                    })
            })
            .collect::<Vec<_>>();

        if nodes.len() < 2 {
            // si soy el unico nodo o no hay nodos, no hay siguiente
            return None;
        }

        // Buscar el siguiente en el anillo
        let idx = nodes.iter().position(|x| *x == self.my_socket_addr)?;
        let next = nodes.get((idx + 1) % nodes.len())?;
        Some(*next)
    }

    fn find_previous_in_ring(&self) -> Option<SocketAddr> {
        // Paso 1: Obtener y ordenar los IDs del anillo
        let mut ordered_ids: Vec<_> = self.ring_nodes.keys().cloned().collect();
        ordered_ids.sort(); // "server_0", "server_1", ...

        // Paso 2: Buscar índice del nodo actual
        let my_index = ordered_ids.iter().position(|id| id == &self.id)?;

        // Paso 3: Buscar el anterior nodo *vivo* en el anillo
        for offset in 1..=ordered_ids.len() {
            let idx = (my_index + ordered_ids.len() - offset) % ordered_ids.len();
            let prev_id = &ordered_ids[idx];

            print!("       prev_id: {}, ", prev_id);

            // Obtener su SocketAddr
            if let Some(addr) = self.coord_addresses.get_by_value(prev_id) {
                // Confirmar que está vivo
                if self.coord_communicators.contains_key(addr) {
                    return Some(*addr);
                } else {
                }
            }
        }

        // Si no encontramos ninguno vivo
        None
    }

    fn start_heartbeat_checker(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(INTERVAL_HEARTBEAT, |act, ctx| {
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
                    //act.heartbeat_timestamps.remove(&leader);

                    act.start_leader_election();
                } else {
                    //act.logger.info("Enviando Ping al líder...");

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
                            act.pong_leader_addr = Some(leader);

                            let addr = ctx.address();
                            let handler = ctx.run_later(TIMEOUT_HEARTBEAT, move |_, _| {
                                addr.do_send(CheckPongTimeout);
                            });
                            act.waiting_pong_timer = Some(handler);
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
                            //act.heartbeat_timestamps.remove(&leader);

                            act.pong_pending = false;
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
        &mut self,
        target: SocketAddr,
        message: NetworkMessage,
    ) -> Result<(), String> {
        if let Some(communicator) = self.coord_communicators.get(&target) {
            if let Some(sender) = &communicator.sender {
                self.logger
                    .info(format!("Sending message to {}: {:?}", target, message));
                match sender.try_send(message) {
                    Ok(_) => {
                        //self.logger.info(format!("✅ Sent successfully to {}", target));
                        Ok(())
                    }
                    Err(e) => {
                        let err_msg = format!("❌ Failed to send to {}: {:?}", target, e);
                        //self.logger.error(&err_msg);

                        // Eliminar nodo por fallo de envío
                        self.coord_communicators.remove(&target);
                        self.coord_addresses.remove_by_key(&target);

                        //self.logger.warn(format!("🧹 Removed unreachable node {}", target));

                        Err(err_msg)
                    }
                }
            } else {
                let err_msg = format!("⚠️ Sender not initialized in communicator for {}", target);
                self.logger.error(&err_msg);

                // También lo removemos, por estar mal configurado
                self.coord_communicators.remove(&target);
                self.coord_addresses.remove_by_key(&target);

                //self.logger.warn(format!("🧹 Removed node with uninitialized sender {}", target));

                Err(err_msg)
            }
        } else {
            let err_msg = format!("❌ No communicator found for {}", target);
            //self.logger.error(&err_msg);

            // El nodo ya está desconectado, aseguramos limpieza por las dudas
            self.coord_addresses.remove_by_key(&target);

            //self.logger.warn(format!("🧹 Removed node with no communicator {}", target));

            Err(err_msg)
        }
    }

    /// Envía un `NetworkMessage` a todos los nodos remotos conectados
    fn broadcast_network_message(&mut self, message: NetworkMessage) {
        for addr in self.coord_communicators.keys().copied().collect::<Vec<_>>() {
            if addr != self.my_socket_addr {
                if let Err(err) = self.send_network_message(addr, message.clone()) {
                    self.logger
                        .error(format!("Failed to send message to {}: {}", addr, err));
                }
            }
        }
    }

    fn broadcast_leader_is(&mut self) {
        if let Some(leader) = self.coordinator_actual {
            if let Some(leader_id) = self.coord_addresses.get_by_key(&leader) {
                let message = NetworkMessage::LeaderIdIs(LeaderIdIs {
                    leader_id: leader_id.to_string(),
                });
                self.logger
                    .info(format!("Broadcasting new leader: {}", leader));
                self.broadcast_network_message(message);
            } else {
                self.logger.warn(format!(
                    "No leader ID found for current coordinator address: {}",
                    leader
                ));
            }
        }
    }

    fn broadcast_who_is_leader(&mut self) -> Result<(), String> {
        if self.coord_communicators.is_empty() {
            return Err("No coordinators available to contact.".to_string());
        }

        let mut sent = false;
        let addrs: Vec<_> = self.coord_communicators.keys().copied().collect();

        for addr in addrs {
            if addr == self.my_socket_addr {
                continue;
            }

            if let Some(communicator) = self.coord_communicators.get(&addr) {
                let local_addr = communicator.local_address;
                let message = NetworkMessage::WhoIsLeader(WhoIsLeader {
                    origin_addr: local_addr,
                    user_id: self.id.clone(),
                });

                match self.send_network_message(addr, message.clone()) {
                    Ok(_) => {
                        sent = true;
                        self.logger
                            .info(format!("Broadcasting WhoIsLeader to {}", addr));
                    }
                    Err(err) => {
                        self.logger
                            .error(format!("Failed to send WhoIsLeader to {}: {}", addr, err));
                    }
                }
            } else {
                self.logger
                    .warn(format!("No communicator found for {}", addr));
            }
        }

        if sent {
            Ok(())
        } else {
            Err("Failed to send WhoIsLeader to any node.".to_string())
        }
    }

    /// Preguntar a todos los nodos conocidos si hay un líder ya elegido
    fn ask_for_leader(&mut self, ctx: &mut Context<Self>) {
        let id = self.id.clone();
        match self.broadcast_who_is_leader() {
            Ok(_) => {
                // Esperamos X segundos para ver si alguien responde
                ctx.run_later(TIMEOUT_LEADER_RESPONSE, |actor: &mut Self, _ctx| {
                    if actor.coordinator_actual.is_none() {
                        actor
                            .logger
                            .info("Asked all nodes for leader. No responses. Becoming leader...");
                        actor.coordinator_actual = Some(actor.my_socket_addr);
                        actor.coordinator_addr.do_send(LeaderIdIs { leader_id: id });
                        actor.broadcast_leader_is();
                    } else {
                        actor.logger.info(format!(
                            "Leader response received before timeout: {:?}",
                            actor.coordinator_actual
                        ));
                        // Nos conectamos por primera vez al lider y solicitamos todo el Storage
                        if let Some(addr) = actor.coordinator_actual {
                            // logea que enviaste
                            actor
                                .logger
                                .info(format!("Requesting all storage from leader at {}", addr));
                            if let Err(e) = actor.send_network_message(
                                addr,
                                NetworkMessage::RequestAllStorage(RequestAllStorage {
                                    coordinator_id: id.clone(),
                                }),
                            ) {
                                actor.logger.warn(format!(
                                    "Error al enviar RequestAllStorage al líder: {}",
                                    e
                                ));
                            }
                        } else {
                            actor
                                .logger
                                .warn("No coordinator address found to request storage.");
                        }
                    }
                });
            }
            Err(reason) => {
                self.logger.warn(format!(
                    "No coordinators to contact for leader election: {}",
                    reason
                ));
                // Nos autoproclamamos líder directamente
                self.coordinator_actual = Some(self.my_socket_addr);
                self.coordinator_addr.do_send(LeaderIdIs { leader_id: id });
                self.broadcast_leader_is();
            }
        }
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
            if let Some(leader_id) = self.coord_addresses.get_by_key(&leader) {
                let response = NetworkMessage::LeaderIdIs(LeaderIdIs {
                    leader_id: leader_id.to_string(),
                });
                if let Some(registered_remote_addr) =
                    self.coord_addresses.get_by_value(&msg.user_id)
                {
                    if let Some(communicator) = self.coord_communicators.get(registered_remote_addr)
                    {
                        if let Some(sender) = &communicator.sender {
                            sender.do_send(response);
                            self.logger
                                .info(format!("Sent LeaderIdIs to {}", msg.origin_addr));
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
                self.logger.warn(format!(
                    "No leader ID found for current coordinator address: {}",
                    leader
                ));
            }
        } else {
            self.logger.info("No coordinator known yet to respond");
        }
    }

    fn handle_leader_is(&mut self, msg: LeaderIdIs, _ctx: &mut Context<Self>) {
        self.election_in_progress = false;
        if self.coordinator_actual.is_none() {
            if let Some(leader_addr) = self.coord_addresses.get_by_value(&msg.leader_id) {
                self.logger.info(format!(
                    "Received LeaderIdIs from {}, updating coordinator to {}",
                    leader_addr, msg.leader_id
                ));
                self.coordinator_actual = Some(*leader_addr); ///////////////
            } else {
                self.logger.info(format!(
                    "Received LeaderIdIs from {}, but no address found for it",
                    msg.leader_id
                ));
            }

            self.coordinator_addr.do_send(LeaderIdIs {
                leader_id: msg.leader_id.clone(),
            });
            self.logger
                .info(format!("Updated local coordinator to {}", msg.leader_id));
        } else if let Some(registered_remote_addr) =
            self.coord_addresses.get_by_value(&msg.leader_id)
        {
            if self.coordinator_actual != Some(*registered_remote_addr) {
                self.logger.warn(format!(
                    "Pisé a mi coordinador porque llegó LeaderIdIs. Local: {:?}, Received: {}",
                    self.coordinator_actual, *registered_remote_addr
                ));
                self.coordinator_actual = Some(*registered_remote_addr); //piso al actual
            }
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
        // Iniciar el chequeo de actualizaciones de Storage
        self.start_storage_updates_checker(ctx);
    }
}

impl Handler<RequestAllStorage> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, msg: RequestAllStorage, ctx: &mut Context<Self>) {
        self.logger.info(format!(
            "Received RequestAllStorage from {}",
            msg.coordinator_id
        ));
        let id = msg.coordinator_id.clone();

        if let Some(remote_addr) = self.coord_addresses.get_by_value(&id) {
            if !self.coord_communicators.contains_key(remote_addr) {
                self.logger.warn(format!(
                    "Coordinador {} no está conectado, no puedo enviarle actualizaciones de Storage",
                    id
                ));
                return;
            }
        } else {
            self.logger.warn(format!(
                "No se encontró dirección para el coordinador: {}",
                id
            ));
            return;
        }

        let storage_addr = self.storage.clone();
        let remote_addr = self
            .coord_addresses
            .get_by_value(&msg.coordinator_id)
            .cloned()
            .unwrap();

        storage_addr
            .send(GetAllStorage)
            .into_actor(self)
            .then(move |res, act, _ctx| {
                match res {
                    Ok(snapshot) => {
                        act.send_network_message(
                            remote_addr,
                            NetworkMessage::StorageSnapshot(StorageSnapshot { snapshot }),
                        )
                        .unwrap_or_else(|e| {
                            act.logger
                                .error(format!("Error al enviar StorageSnapshot: {}", e))
                        });
                    }
                    Err(e) => {
                        act.logger
                            .warn(format!("Error al obtener snapshot de storage: {:?}", e));
                    }
                }
                fut::ready(())
            })
            .spawn(ctx);
    }
}

impl Handler<WhoIsLeader> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, msg: WhoIsLeader, ctx: &mut Context<Self>) {
        // imprimir los comunicators que tenga
        self.handle_who_is_leader(msg, ctx);
    }
}

impl Handler<LeaderIdIs> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, msg: LeaderIdIs, _ctx: &mut Context<Self>) {
        self.logger
            .info(format!("Líder recibido: {}", msg.leader_id));
        self.handle_leader_is(msg, _ctx);
    }
}

impl Handler<CheckPongTimeout> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, _msg: CheckPongTimeout, _ctx: &mut Self::Context) {
        if self.pong_pending && self.pong_leader_addr == self.coordinator_actual {
            self.logger
                .warn("Timeout esperando Pong. Iniciando elección...");
            self.pong_pending = false;

            if let Some(dead_leader) = self.coordinator_actual {
                self.coord_communicators.remove(&dead_leader);
                self.coord_addresses.remove_by_key(&dead_leader);
                //self.heartbeat_timestamps.remove(&dead_leader);
            }

            self.coordinator_actual = None;
            self.start_leader_election();
        } else {
            //self.logger.info("⚠️ CheckPongTimeout ignorado: ya no esperamos Pong del coordinador actual.");
        }
    }
}

impl Handler<Ping> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, msg: Ping, _ctx: &mut Self::Context) {
        //self.logger.info(format!("Recibido Ping de {}", msg.from));

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

    fn handle(&mut self, tokio_stream: Pong, _ctx: &mut Self::Context) {
        //self.logger.info(format!("Recibido Pong de {}", msg.from));
        // Pong recibido, ya no hay ping pendiente
        self.pong_pending = false;
    }
}

impl Handler<LeaderElection> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, msg: LeaderElection, _ctx: &mut Self::Context) {
        let mut candidates = msg.candidates.clone();

        if msg.initiator == self.id {
            // Completó el ciclo
            self.election_in_progress = false;
            let new_leader = candidates.iter().min().unwrap().clone();
            self.logger
                .info(format!("Elección terminada. Nuevo líder: {}", new_leader));

            if let Some(leader_addr) = self.coord_addresses.get_by_value(&new_leader) {
                self.coordinator_actual = Some(*leader_addr);
                self.broadcast_leader_is();
            } else {
                self.logger.warn(format!(
                    "No se encontró dirección para el nuevo líder: {}",
                    new_leader
                ));
            }
        } else {
            // Sumarme como candidato
            candidates.push(self.id.clone());
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

impl Handler<StorageUpdates> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, msg: StorageUpdates, _ctx: &mut Context<Self>) {
        self.logger.info(format!(
            "Recibidas {} actualizaciones de Storage, reenviando a storage...",
            msg.updates.len()
        ));
        // HashMap -> Vector en orden en base a los keys (indices)
        let mut updates_vec: Vec<_> = msg.updates.into_iter().collect();
        updates_vec.sort_by_key(|(index, _)| *index);
        // Enviar los logs al storage
        self.storage.do_send(ApplyStorageUpdates {
            is_leader: self.coordinator_actual == Some(self.my_socket_addr),
            updates: updates_vec,
        });
    }
}

impl Handler<StorageSnapshot> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, msg: StorageSnapshot, _ctx: &mut Context<Self>) {
        self.storage.do_send(msg);
    }
}

impl Handler<RequestNewStorageUpdates> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, msg: RequestNewStorageUpdates, _ctx: &mut Context<Self>) {
        // lo buscamos en el bimap de direcciones y luego en comunicadores. Si no esta en comunicadores, no lo tenemos conectado
        let id = msg.coordinator_id.clone();

        if let Some(remote_addr) = self.coord_addresses.get_by_value(&id) {
            if !self.coord_communicators.contains_key(remote_addr) {
                self.logger.warn(format!(
                    "Coordinador {} no está conectado, no puedo enviarle actualizaciones de Storage",
                    id
                ));
                return;
            }
        } else {
            self.logger.warn(format!(
                "No se encontró dirección para el coordinador: {}",
                id
            ));
            return;
        }

        self.logger.info(format!(
            "Recibida solicitud de actualizaciones de Storage desde el nodo {}",
            msg.start_index
        ));

        // Enviar las actualizaciones de Storage al nodo que lo solicitó
        self.storage
            .send(GetLogsFromIndex {
                index: msg.start_index,
            })
            .into_actor(self)
            .then(move |res, act, _ctx| {
                match res {
                    Ok(updates) => {
                        // Aquí deberíamos enviar las actualizaciones desde min_log_index hasta el final
                        // Por simplicidad, enviamos un mensaje vacío

                        let remote_addr = act
                            .coord_addresses
                            .get_by_value(&msg.coordinator_id)
                            .cloned()
                            .unwrap();

                        act.send_network_message(
                            remote_addr,
                            NetworkMessage::StorageUpdates(StorageUpdates { updates }),
                        )
                        .unwrap_or_else(|e| {
                            act.logger
                                .error(format!("Error al enviar StorageUpdates: {}", e))
                        });
                    }
                    Err(e) => {
                        act.logger
                            .warn(format!("Error al obtener minLogIndex de storage: {:?}", e));
                    }
                }
                fut::ready(())
            })
            .spawn(_ctx);
    }
}

impl Handler<ConnectionClosed> for CoordinatorManager {
    type Result = ();

    fn handle(&mut self, msg: ConnectionClosed, ctx: &mut Self::Context) {
        self.logger
            .info(format!("Connection closed: {}", msg.remote_addr));
        // Eliminar el comunicador y la dirección del nodo
        self.coord_communicators.remove(&msg.remote_addr);

        // Preguntar a Agus por qué lo comenté
        // self.coord_addresses.remove_by_key(&msg.remote_addr);

        // Lo reemplacé con esto de acá:
        if let Some(closed_id) = self.coord_addresses.get_by_key(&msg.remote_addr).cloned() {
            if let Some(acceptor_addr) = self.ring_nodes.get(&closed_id) {
                // Actualizar el address del nodo cerrado al del aceptado
                self.coord_addresses
                    .insert(*acceptor_addr, closed_id.clone());
                self.logger.info(format!(
                    "Actualizado address de {} a {}",
                    closed_id, *acceptor_addr
                ));
            } else {
                self.logger
                    .warn("No previous node found to update address.");
            }
        } else {
            self.logger.warn(format!(
                "No ID found for closed node at address: {}",
                msg.remote_addr
            ));
        }

        // Si el nodo cerrado era el líder actual, iniciamos una elección
        if self.coordinator_actual == Some(msg.remote_addr) {
            self.logger.warn(format!(
                "Líder caído: {}. Iniciando elección...",
                msg.remote_addr
            ));
            self.coordinator_actual = None;
            self.election_in_progress = true;

            if let Some(handle) = self.waiting_pong_timer.take() {
                ctx.cancel_future(handle);
                self.waiting_pong_timer = None;
                self.pong_pending = false;
                self.pong_leader_addr = None;
            }

            self.start_leader_election();
        }

        //self.heartbeat_timestamps.remove(&msg.addr);
    }
}
