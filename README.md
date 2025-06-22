<p align="center">
  <img src=img/logo_pedidos_rust.png width="350" alt="Logo PedidosRust">
</p>

# Programacion Concurrente - 2C2025 - PedidosRust

[![Review Assignment Due Date](https://classroom.github.com/assets/deadline-readme-button-22041afd0340ce965d47ae6ef1cefeee28c7c493a6346c4f15d667ab976d596c.svg)](https://classroom.github.com/a/YmMajyCa)

**PedidosRust** es un sistema distribuido implementado en Rust que modela la interacción entre _clientes_, _restaurantes_, _repartidores_ y un _gateway de pagos_. Cada entidad funciona como una aplicación independiente, comunicándose mediante mensajes TCP.

La consigna del trabajo práctico puede encontrarse [aqui](https://concurrentes-fiuba.github.io/2025_1C_tp2.html)

---

## Autores

| Nombre         | Apellido      | Mail                  | Padrón |
| -------------- | ------------- | --------------------- | ------ |
| Ian            | von der Heyde | ivon@fi.uba.ar        | 107638 |
| Agustín        | Altamirano    | aaltamirano@fi.uba.ar | 110237 |
| Juan Martín    | de la Cruz    | jdelacruz@fi.uba.ar   | 109588 |
| Santiago Tomás | Fassio        | sfassio@fi.uba.ar     | 109463 |

---

## Índice

1. [Diseño del sistema](#diseño-del-sistema)
   - [Características principales](#características-principales)
   - [Procesos del sistema](#procesos-del-sistema)
   - [Modelo de comunicación entre procesos](#modelo-de-comunicación-entre-procesos-tcp-sender-y-tcp-receiver)
   - [Actores y mensajes por cada proceso](#actores-por-proceso)
     - [Proceso Server](#proceso-server)
     - [Proceso PaymentGateway](#proceso-paymentgateway)
     - [Proceso Cliente](#proceso-cliente)
     - [Proceso Restaurante](#proceso-restaurante-async)
     - [Proceso Delivery](#proceso-delivery-async)
   - [Modelo de replicación y tolerancia a fallos](#modelo-de-replicación-de-servidores-y-tolerancia-a-fallos)
   - [Elección de líder](#elección-de-líder)
2. [Cambios en la entrega final](#cambios-en-la-entrega-final)
   - [Heartbeats](#heartbeats)
   - [Nuevos Mensajes implementados](#nuevos-mensajes-implementados)
3. [Instalación y Ejecución](#instalación-y-ejecución)
4. [Ejemplo de Ejecución](#ejemplo-de-ejecución)
5. [Pruebas](#pruebas)

---

## **Diseño del sistema**

### **Características principales**

- **Modelo de Actores Asincrónicos**
  El sistema está construido siguiendo el **modelo de actores**, lo que permite una gestión eficiente y concurrente de mensajes entre múltiples entidades distribuidas. Cada componente del sistema (clientes, restaurantes, repartidores, servidores) está representado por actores independientes que se comunican de forma no bloqueante a través de TCP.

- **Coordinación distribuida y elección de coordinador**
  Se implementa el **algoritmo del anillo (Ring Algorithm)** para llevar a cabo la **elección de un Coordinator Manager** entre los distintos procesos `Coordinator`. Este mecanismo garantiza que, ante la caída del coordinador actual, el sistema pueda elegir automáticamente un nuevo líder sin necesidad de intervención externa.

- **Exclusión Mutua Distribuida (Centralizada)**
  Cuando se termina de preparar un pedido, el `Coordinator`, particularmente el actor `Storage`, debe asegurarse de que no se produzcan conflictos al asignar un delivery. Para ello, este actúa como punto de **exclusión mutua centralizada**. El `DeliveryAssigner` le envía un mensaje `RequestDelivery` al servidor, el cual notifica a todos los repartidores cercanos que hay un pedido listo para entregar enviándoles el mensaje `NewOfferToDeliver`. Los repartidores interesados envían un mensaje de oferta al servidor (`DeliveryAccepted`), el servidor redirige la solicitud al `OrderService` reenviándole el mensaje `DeliveryAccepted` y este último selecciona al primero que se haya ofrecido, asignándole el pedido. En caso de que el `OrderService` recibe un mensaje de aceptacion del pedido, consulta con el storage el estado actual de la orden. En caso de que la orden ya estuviera tomada por otro delivery, le envía un mensaje para comunicarle que el delivery ya fue tomado [`DeliveryNoNeeded`]. Luego, el servidor envía al restaurant ese delivery disponible con el mensaje `DeliveryAvailable`. El restaurant enviará el mensaje `DeliverThisOrder`, el cual se reenviará al repartidor asignado para que complete la entrega. El resto de los repartidores reciben una notificación (mensaje `DeliveryNoNeeded`) de que ya no es necesario que se ofrezcan para esa entrega.

- **Serialización de los accesos al estado global**
  Dentro del servidor, se encuentra el actor `Storage`, el cual es responsable de almacenar y gestionar el estado global del sistema. Este actor actúa como un repositorio centralizado para la información de clientes, restaurantes, repartidores y órdenes, asegurando que todos los nodos tengan acceso a un estado consistente. Al tratarse de un actor, el acceso a `Storage` está protegido por el modelo de actores, lo que evita problemas de concurrencia, permitiendo que múltiples nodos (clientes, restaurantes, deliveries y el gateway de pagos) interactúen con el estado global sin producirse race conditions.

- **Resiliencia y Tolerancia a Fallos**
  El sistema está diseñado con foco en la **tolerancia a fallos**, permitiendo que nodos individuales (como clientes, repartidores o restaurantes) puedan desconectarse temporalmente **sin afectar el flujo global del sistema**. Esta resiliencia se logra mediante:

  - **Heartbeats periódicos** entre procesos `Coordinator`, para detectar y responder rápidamente ante fallas.
  - **Backups sincronizados** del estado del sistema, asegurando persistencia y recuperación consistente.
  - **Soporte para reconexión de nodos**: los procesos pueden reconectarse automáticamente. Además, según el **estado actual de la orden**, es posible que ciertas operaciones (como la entrega de un pedido) continúen exitosamente **incluso si un cliente u otro nodo se encuentra momentáneamente desconectado**.

---

### **Procesos del Sistema**

El sistema está conformado por múltiples procesos independientes que se ejecutan en consolas separadas. Cada proceso representa un **nodo autónomo** dentro de la arquitectura distribuida del sistema, y se comunica mediante **mensajes TCP asincrónicos**.

#### Procesos principales

Los siguientes procesos representan las distintas funciones centrales del sistema:

- **PaymentGateway** — Puerto TCP: `8080`
- **Server1** — Puerto TCP: `8081`
- **Server2** — Puerto TCP: `8082`
- **Server3** — Puerto TCP: `8083`
- **Server4** — Puerto TCP: `8084`

Cada uno de estos servidores ejecuta un `Coordinator`, coordina actores internos y maneja conexiones con otros nodos del sistema.

#### Procesos dinámicos

Además, por cada entidad de negocio se lanza un proceso independiente:

- **Cliente** — Un proceso por cada cliente activo.
- **Restaurante** — Un proceso por cada restaurante disponible.
- **Delivery** — Un proceso por cada repartidor conectado.

Estos procesos se conectan dinámicamente a alguno de los `Server`, y se comunican de forma bidireccional para operar dentro del sistema (por ejemplo, iniciar pedidos, aceptar entregas, recibir actualizaciones, etc.).

---

### Actores por proceso

Cada proceso está compuesto por varios actores, cada uno con una responsabilidad específica. A continuación se describen los actores de cada proceso:

- [**Proceso Server**](#proceso-server):

  - Acceptor
  - N Communicators -> (TCPSender, TCPReceiver)
  - Coordinator
  - CoordinatorManager
  - OrderService
  - NearbyDeliveryService
  - NearbyRestaurantService
  - Storage
  - Reaper

- [**Proceso PaymentGateway**](#proceso-paymentgateway):

  - Acceptor
  - PaymentGateway
  - N Communicators -> (TCPSender, TCPReceiver)

- [**Proceso Cliente**](#proceso-cliente):

  - Client
  - UIHandler
  - Communicator -> (TCPSender, TCPReceiver)

- [**Proceso Restaurante**](#proceso-restaurante):

  - Restaurant
  - Kitchen
  - Chef
  - DeliveryAssigner
  - Communicator -> (TCPSender, TCPReceiver)

- [**Proceso Delivery**](#proceso-delivery):
  - TCP Sender
  - TCP Receiver
  - Delivery
  - Communicator -> (TCPSender, TCPReceiver)

---

### Modelo de comunicación entre procesos: `TCP Sender` y `TCP Receiver`

La comunicación entre procesos distribuidos en este sistema se realiza a través de **mensajes TCP**. Para abstraer esta comunicación y mantener la lógica del sistema desacoplada del transporte subyacente, se utilizan dos actores especializados:

#### 📤 `TCPSender` _(Async)_

El `TCPSender` es el actor responsable de **enviar mensajes TCP** hacia otro nodo del sistema.

```rust
pub struct TCPSender {
    /// Writer con buffer para el stream TCP.
    pub writer: Option<BufWriter<WriteHalf<TcpStream>>>,
    /// Cola de mensajes pendientes por enviar.
    pub queue: VecDeque<NetworkMessage>,
}
```

Características:

- Utiliza un `BufWriter` sobre la mitad de escritura del socket (`WriteHalf<TcpStream>`).
- Recibe mensajes desde otros actores del sistema (por ejemplo, `Coordinator`, `Client`, etc.) y los escribe en el socket.
- Está diseñado para trabajar en paralelo con un `TCPReceiver` que lee de la misma conexión.

#### 📥 `TCPReceiver` _(Async)_

El `TCPReceiver` es el actor responsable de **leer mensajes entrantes desde un socket TCP** y reenviarlos al actor de destino adecuado dentro del sistema.

```rust
pub struct TCPReceiver<A: Actor + Handler<NetworkMessage>> {
    /// La dirección de socket del par remoto.
    remote_addr: SocketAddr,
    /// El lector con búfer para el flujo TCP.
    reader: Option<BufReader<ReadHalf<TcpStream>>>,
    /// La dirección Actix del actor de destino.
    destination: Addr<A>,
}
```

Características:

- Utiliza un `BufReader` sobre la mitad de lectura del socket (`ReadHalf<TcpStream>`).
- Deserializa cada línea recibida y la envía como mensaje al actor indicado mediante `destination`.
- Es genérico en cuanto al actor destino, lo que permite reutilizarlo en múltiples procesos (por ejemplo, `Client`, `Restaurant`, etc.).

#### 🔄 Emparejamiento mediante `Communicator`

Tanto el `TCP Sender` como el `TCP Receiver` están encapsulados dentro de una estructura llamada `Communicator`, que representa una **conexión lógica con otro nodo** (cliente, restaurante, delivery, otro servidor, o el Payment Gateway).

```rust
pub struct Communicator<A>
where
    A: Actor<Context = Context<A>> + Handler<NetworkMessage>,
{
    /// La dirección de socket local de este par.
    pub local_address: SocketAddr,
    /// La dirección de socket del par remoto.
    pub peer_address: SocketAddr,
    /// El actor TCP remitente para los mensajes salientes.
    pub sender: Option<Arc<Addr<TCPSender>>>,
    /// El actor TCP receptor para los mensajes entrantes.
    pub receiver: Option<Arc<Addr<TCPReceiver<A>>>>,
    /// El tipo del par remoto.
    pub peer_type: PeerType, // Enum: Cliente, Restaurante, Entrega, Coordinador, Gateway
}

```

Este diseño permite que los distintos actores del sistema interactúen entre sí mediante mensajes, sin necesidad de preocuparse por la gestión directa de sockets o serialización.

---

### **Proceso `Server`**

Cada proceso `Server` representa un nodo del sistema. Cada uno de estos procesos se ejecuta en una consola diferente y se comunica a través de mensajes TCP.

<p align="center">
  <img src="img/server_architecture.png" style="max-width: 100%; height: auto;" alt="Arquitectura del server">
</p>

A continuación, desarrollaremos en base al proceso `Server1` como ejemplo, pero el funcionamiento es el mismo para los otros procesos `Server`.

---

#### 🔌 **Acceptor** _(Async)_

El actor **Acceptor** es responsable de escuchar el puerto TCP del proceso `Server`, aceptando conexiones entrantes desde diversos tipos de nodos del sistema: clientes, restaurantes, repartidores, otros servidores (`CoordinatorX`) y el `Payment Gateway`.

Por cada nueva conexión aceptada, se instancian automáticamente los siguientes actores de comunicación:

- 📤 [`TCPSender`](#comunicación-entre-procesos-tcp-sender-y-tcp-receiver)
- 📥 [`TCPReceiver`](#comunicación-entre-procesos-tcp-sender-y-tcp-receiver)

Estos actores son los encargados de gestionar la entrada y salida de mensajes TCP entre el `Server` y el nodo conectado, desacoplando así la lógica de transporte del resto del sistema.

##### Estado interno del actor Acceptor

```rust
pub struct Acceptor {
    /// La dirección a la que se debe enlazar y escuchar conexiones entrantes.
    addr: SocketAddr,
    /// La dirección del actor coordinador con el que registrar las conexiones.
    coordinator_address: Addr<Coordinator>,
    /// Registrador de eventos del acceptor.
    logger: Logger,
}
```

---

#### 🧠 **Coordinator** _(Async)_

El actor **Coordinator** es el **componente central de coordinación** del proceso `Server`. Su función principal es recibir, interpretar y direccionar todos los mensajes entrantes del sistema.

Responsabilidades:

- Recibir mensajes provenientes de los `TCPReceiver`.
- Enviar mensajes hacia los `TCPSender` asociados a clientes, restaurantes, repartidores y al `Payment Gateway`.
- Coordinar acciones con los actores internos:

  - [`CoordinatorManager`](#🔗-coordinatormanager-async)
  - [`OrderService`](#️⚙️-servicios-internos-async)
  - [`NearbyDeliveryService`](#️⚙️-servicios-internos-async)
  - [`NearbyRestaurantService`](#️⚙️-servicios-internos-async)
  - [`Storage`](#🗄️-storage-async)
  - [`Reaper`](#💀-reaper-async)

##### Estado interno del actor Coordinator

```rust
pub struct Coordinator {
    /// Identificador único para este coordinador.
    pub id: String,
    /// Direcciones de todos los nodos en el anillo.
    pub ring_nodes: HashMap<String, SocketAddr>,
    /// Dirección de socket de este coordinador.
    pub my_addr: SocketAddr,
    /// Dirección del coordinador actual (líder).
    pub current_coordinator: Option<SocketAddr>,
    /// Mapa bidireccional de direcciones de usuario e IDs de usuario.
    pub user_addresses: BiMap<SocketAddr, String>,
    /// Mapa de direcciones remotas a sus comunicadores.
    pub communicators: HashMap<SocketAddr, Communicator<Coordinator>>,
    /// Dirección del actor de almacenamiento.
    pub storage: Option<Addr<Storage>>,
    /// Dirección del actor del servicio de pedidos.
    pub order_service: Option<Addr<OrderService>>,
    /// Dirección del actor del servicio de restaurantes cercanos.
    pub nearby_restaurant_service: Option<Addr<NearbyRestaurantsService>>,
    /// Dirección del actor del servicio de repartidores cercanos.
    pub nearby_delivery_service: Option<Addr<NearbyDeliveryService>>,
    /// Reaper para eliminar usuarios inactivos.
    pub reaper: Option<Addr<Reaper>>,
    /// Logger para eventos del coordinador.
    pub logger: Logger,
    /// Dirección del actor gestor de coordinadores.
    pub coordinator_manager: Option<Addr<CoordinatorManager>>,
    /// Streams TCP pendientes para conexiones del anillo.
    pub pending_streams: HashMap<SocketAddr, TcpStream>,
    /// Temporizadores para timeouts de asignación de pedidos.
    pub order_timers: HashMap<u64, SpawnHandle>,
}
```

---

#### 🔗 **CoordinatorManager** _(Async)_

El actor **CoordinatorManager** es el encargado de la **coordinación distribuida entre instancias del proceso `Server`** (Coordinators).

Este actor utiliza los `Communicator` previamente establecidos con `Coordinator2`, `Coordinator3` y `Coordinator4` para implementar:

- El algoritmo de **anillo (ring)** para la organización lógica de los servidores y elección de líder.
- Envío de **heartbeats** para detectar fallos.
- Sincronización periódica del estado del sistema (`Storage`) entre nodos.

##### Estado interno del actor CoordinatorManager

```rust
pub struct CoordinatorManager {
    /// ID único de este CoordinatorManager.
    pub id: String,
    /// Direcciones de todos los nodos en el anillo.
    pub ring_nodes: HashMap<String, SocketAddr>,
    /// Nodo coordinador actual (líder).
    pub coordinator_actual: Option<SocketAddr>,
    /// Mapa de direcciones de nodos coordinadores a sus comunicadores.
    pub coord_communicators: HashMap<SocketAddr, Communicator<Coordinator>>,
    /// Mapa bidireccional de direcciones de coordinadores y sus IDs.
    pub coord_addresses: BiMap<SocketAddr, String>,
    /// Dirección de socket de este servidor.
    pub my_socket_addr: SocketAddr,
    /// Logger para eventos del gestor de coordinadores.
    pub logger: Logger,
    /// Dirección del actor local `Coordinator`.
    pub coordinator_addr: Addr<Coordinator>,
    /// Indica si hay un Pong pendiente del líder.
    pong_pending: bool,
    /// Indica si hay una elección de líder en curso.
    election_in_progress: bool,
    /// Dirección del actor `Storage`.
    pub storage: Addr<Storage>,
    /// Dirección del nodo del que se espera un Pong.
    pong_leader_addr: Option<SocketAddr>,
    /// Handle del temporizador para esperar respuestas Pong.
    waiting_pong_timer: Option<actix::SpawnHandle>,
    /// Handle del temporizador para actualizaciones periódicas de almacenamiento.
    get_storage_updates_timer: Option<actix::SpawnHandle>,
    /// Esperando respuesta del líder.
    waiting_for_leader: Option<actix::SpawnHandle>,
}
```

---

#### ⚙️ **Servicios internos** _(Async)_

Los servicios internos se encargan de tareas especializadas dentro del proceso `Server`, accediendo al actor `Storage` para realizar lecturas y actualizaciones consistentes.

- **OrderService**
  Mantiene el estado de las órdenes en curso.
  Se comunica con: `Coordinato`, `Storage`.

- **NearbyRestaurantService**
  Identifica restaurantes cercanos a un cliente para iniciar el proceso de pedido.
  Se comunica con: `Coordinator`, `Storage`.

- **NearbyDeliveryService**
  Encuentra repartidores disponibles próximos a un restaurante para asignar la entrega.
  Se comunica con: `Coordinator`, `Storage`.

##### Estado interno de OrderService

```rust
pub struct OrderService {
    /// Rastrea el estado de cada pedido por ID de pedido.
    pub orders: HashMap<u64, OrderStatus>,
    /// Mapea los IDs de clientes a sus IDs de pedidos asociados.
    pub clients_orders: HashMap<String, Vec<u64>>,
    /// Mapea los IDs de restaurantes a sus IDs de pedidos asociados.
    pub restaurants_orders: HashMap<String, Vec<u64>>,
    /// Lista de IDs de pedidos pendientes.
    pub pending_orders: Vec<u64>,
    /// Dirección del actor Coordinator.
    pub coordinator_address: Option<Addr<Coordinator>>,
    /// Dirección del actor Storage.
    pub storage_address: Option<Addr<Storage>>,
    /// Logger para eventos del servicio de pedidos.
    pub logger: Logger,
    /// Comunicador para interactuar con el PaymentGateway.
    pub payment_gateway_address: Option<Communicator<OrderService>>,
    /// Stream TCP pendiente para la conexión con PaymentGateway.
    pub pending_stream: Option<TcpStream>,
}
```

##### Estado interno de NearbyDeliveryService

```rust
pub struct NearbyDeliveryService {
    /// Dirección del actor Coordinator al que se enviarán mensajes.
    pub coordinator_address: Addr<Coordinator>,
    /// Dirección del actor Storage para obtener repartidores.
    pub storage_address: Addr<Storage>,
    /// Instancia de logger para eventos.
    pub logger: Logger,
}
```

##### Estado interno de NearbyRestaurantService

```rust
pub struct NearbyRestaurantsService {
    /// Dirección del actor Storage para obtener restaurantes.
    pub storage_addr: Addr<Storage>,
    /// Dirección del actor Coordinator al que se enviarán mensajes.
    pub coordinator_addr: Addr<Coordinator>,
    /// Instancia de logger para eventos.
    pub logger: Logger,
}
```

---

#### 🗄️ **Storage** _(Async)_

El actor **Storage** es responsable de la **persistencia del estado global** del sistema. Administra en memoria la información de entidades del sistema y permite acceder a ellas de forma segura y eficiente.

Gestiona:

- Información de clientes, restaurantes y repartidores.
- Estado detallado de cada orden.

Se comunica directamente con los siguientes actores:

- `Coordinator`
- `OrderService`
- `NearbyDeliveryService`
- `NearbyRestaurantService`

##### Estado interno del storage actor

```rust
/// Objeto de transferencia de datos (DTO) para representar diferentes tipos de usuarios en el sistema.
pub enum UserDTO {
    /// Representa un Cliente (usuario que realiza pedidos).
    Client(ClientDTO),
    /// Representa un Restaurante (usuario que prepara los pedidos).
    Restaurant(RestaurantDTO),
    /// Representa un Repartidor (usuario que lleva los pedidos a los clientes).
    Delivery(DeliveryDTO),
}

pub struct ClientDTO {
    /// Posición del cliente en coordenadas 2D.
    pub client_position: (f32, f32),
    /// ID único del cliente.
    pub client_id: String,
    /// Pedido asociado al cliente (si existe).
    pub client_order: Option<OrderDTO>,
    /// Marca temporal que registra la última actualización del cliente.
    pub time_stamp: std::time::SystemTime,
}

pub struct RestaurantDTO {
    /// Posición del restaurante en coordenadas 2D.
    pub restaurant_position: (f32, f32),
    /// ID único del restaurante.
    pub restaurant_id: String,
    /// Pedidos que el sistema de pago ha autorizado para el restaurante, pero que aún no han sido aceptados por el restaurante.
    pub authorized_orders: HashSet<OrderDTO>,
    /// Pedidos pendientes que el restaurante aún no ha preparado.
    pub pending_orders: HashSet<OrderDTO>,
    /// Marca temporal que registra la última actualización del restaurante.
    pub time_stamp: std::time::SystemTime,
}

pub struct DeliveryDTO {
    /// Posición del repartidor en coordenadas 2D.
    pub delivery_position: (f32, f32),
    /// ID único del repartidor.
    pub delivery_id: String,
    /// ID único del cliente que está siendo atendido actualmente por el repartidor (None si no está disponible).
    pub current_client_id: Option<String>,
    /// ID único del pedido que está siendo entregado (None si no está siendo entregado).
    pub current_order: Option<OrderDTO>,
    /// Estado del repartidor.
    pub status: DeliveryStatus,
    /// Marca temporal que registra la última actualización del repartidor.
    pub time_stamp: std::time::SystemTime,
}

pub struct OrderDTO {
    /// ID único del pedido.
    pub order_id: u64,
    /// Nombre del plato asociado con el pedido.
    pub dish_name: String,
    /// ID único del cliente que realizó el pedido.
    pub client_id: String,
    /// ID único del restaurante que preparará el pedido.
    pub restaurant_id: String,
    /// ID único del repartidor asignado para entregar el pedido (None si no está siendo entregado).
    pub delivery_id: Option<String>,
    /// Estado del pedido.
    pub status: OrderStatus,
    /// Posición del cliente en coordenadas 2D.
    pub client_position: (f32, f32),
    /// Tiempo estimado para la entrega del pedido.
    pub expected_delivery_time: u64,
    /// Marca temporal que registra la última actualización del pedido.
    pub time_stamp: std::time::SystemTime,
}

pub struct Storage {
    /// Diccionario con la información de los clientes.
    pub clients: HashMap<String, ClientDTO>,
    /// Diccionario con la información de los restaurantes.
    pub restaurants: HashMap<String, RestaurantDTO>,
    /// Diccionario con la información de los repartidores.
    pub deliverys: HashMap<String, DeliveryDTO>,
    /// Diccionario de pedidos.
    pub orders: HashMap<u64, OrderDTO>,
    /// Parejas de Repartidores y sus pedidos asignados.
    pub accepted_deliveries: BiMap<u64, String>,
    /// Lista de actualizaciones de registros de almacenamiento.
    pub storage_updates: HashMap<u64, StorageLogMessage>,
    /// Índice de la siguiente entrada en el registro.
    pub next_log_id: u64,
    /// Índice de la operación mínima persistente en el registro.
    pub min_persistent_log_index: u64,
    /// Dirección del `Coordinator` asociado.
    pub coordinator: Addr<Coordinator>,
    /// Registrador de eventos de almacenamiento.
    pub logger: Logger,
}
```

---

#### 💀 **Reaper** _(Async)_

El actor **Reaper** escucha mensajes del `Coordinator` sobre desconexiones, y espera un tiempo antes de eliminar definitivamente a un usuario desconectado que no se reconectó todavía.

Responsabilidades:

1. Recibir mensajes `ReapUser` desde el `Coordinator` con información del usuario desconectado.
2. Iniciar un temporizador de ciertos segundos por cada entidad.
3. Al finalizar el temporizador, reenviar el mismo mensaje `ReapUser` al `Storage` para que decida si debe eliminarlo (basado en su timestamp más reciente).

##### Estado interno de `Reaper`

```rust
pub struct Reaper {
    /// Un mapa de IDs de usuario a sus manejadores de temporizador asociados.
    pub users_timer: HashMap<String, SpawnHandle>,
    /// La dirección del actor de almacenamiento a la que enviar los mensajes.
    pub storage_addr: Addr<Storage>,
}
```

#### Tabla de estados del usuario (desde la perspectiva del Reaper)

| Estado Inicial      | Evento o Acción                       | Estado Final        | Actor Responsable      | Comentario                                                      |
| ------------------- | ------------------------------------- | ------------------- | ---------------------- | --------------------------------------------------------------- |
| `CONECTADO`         | Socket se cierra                      | `PENDIENTE_DE_REAP` | `Coordinator → Reaper` | El coordinator detecta desconexión y lo reporta al Reaper.      |
| `PENDIENTE_DE_REAP` | Usuario no se reconecta en 10s        | `ELIMINADO`         | `Reaper → Storage`     | Se verifica si hubo reconexión; si no, se elimina la entidad.   |
| `PENDIENTE_DE_REAP` | Usuario se reconecta antes de los 10s | `RECUPERADO`        | `Storage`              | El Storage detecta un timestamp más reciente y no elimina nada. |

---

### Mensajes del Proceso `Server` (CoordinatorManager, Coordinator, TCP, Servicios)

#### Elección de Líder y sincronización entre coordinadores

| Mensaje                                                                  | Emisor                                  | Receptor                                             | Descripción                                                                                                                                        |
| ------------------------------------------------------------------------ | --------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `WhoIsLeader`                                                            | `CoordinatorManagerX`                   | Todos los `CoordinatorManager`                       | Pregunta quién es el líder actual. Si no hay respuesta, se autoproclama con `LeaderIs`.                                                            |
| `WhoIsLeader`                                                            | `Client` / `Restaurant` / `Delivery`    | Todos los `Coordinators`                             | Preguntan quién es el líder actual del sistema.                                                                                                    |
| `LeaderIs(SocketAddr)`                                                   | `Coordinator`                           | `Client` / `Restaurant` / `Delivery` / `Coordinator` | Respuesta que informa que el líder es el `Coordinator` con dirección `SocketAddr`.                                                                 |
| `RequestNewStorageUpdates(u64)`                                          | `CoordinatorManagerX`                   | `CoordinatorManagerY`                                | Solicita actualizaciones (a partir de un determinado índice de operación) para sincronizar los datos almacenados.                                  |
| `StorageUpdates(HashMap<u64, Message>)`                                  | `CoordinatorManagerY`                   | `CoordinatorManagerX`                                | Respuesta con los mensajes de actualización del `Storage` restantes para estar actualizado.                                                        |
| `ApplyStorageUpdates(HashMap<u64, Message>)`                             | `CoordinatorManagerX`                   | `StorageX`                                           | Aplica los cambios del `Storage` para mantenerlo actualizado.                                                                                      |
| `RequestAllStorage`                                                      | `CoordinatorManagerX` (recién iniciado) | `CoordinatorManagerY`                                | Solicita las operaciones necesarias para reconstruir todo el `Storage` actual.                                                                     |
| `RecoverStorageOperations(HashMap<u64, Message>, HashMap<u64, Message>)` | `CoordinatorManagerY`                   | `CoordinatorManagerX` (recién creado)                | Respuesta que contiene tanto operaciones necesarias para reconstruir todo el `Storage` actual como la totalidad del registro de operaciones actual |
| `SetStorageUpdatesLog(HashMap<u64, Message>)`                            | `CoordinatorManagerX` (recién creado)   | `StorageX`                                           | Establece el registro de operaciones con el diccionario del payload                                                                                |
| `LeaderElection(Vec<SocketAddr>)`                                        | `CoordinatorManagerX`                   | `CoordinatorManagerY`                                | Propaga por el anillo las IDs (`SocketAddr`) de los `Coordinator` candidatos a líder                                                               |
| `StartReapProcess(UserID)`                                               | `Coordinator`                           | `Reaper`                                             | Notifica que el socket asociado a un usuario se cerró; posible desconexión.                                                                        |
| `CheckReapUser(UserId)`                                                  | `Reaper`                                | `Storage`                                            | Verifica si el usuario desconectado debe eliminarse (por tiempo).                                                                                  |
| `ForwardMessage(SocketAddr, Message)`                                    | `TCPReceiver`                           | `Coordinator` / `CoordinatorManager`                 | Encapsula y reenvía mensajes externos entrantes.                                                                                                   |
| `SendToSocket(Message)`                                                  | `Coordinator` / `CoordinatorManager`    | `TCPSender`                                          | Solicita enviar un mensaje al socket TCP correspondiente.                                                                                          |

---

#### Pedidos y Asignaciones

| Mensaje                                 | Emisor                               | Receptor                             | Descripción                                                                                        |
| --------------------------------------- | ------------------------------------ | ------------------------------------ | -------------------------------------------------------------------------------------------------- |
| `RegisterUser(UserDTO)`                 | `Client` / `Restaurant` / `Delivery` | `Coordinator`                        | Un usuario se conecta; se registra.                                                                |
| `RecoveredUserInfo(Option<UserDTO>)`    | `Coordinator`                        | `Client` / `Restaurant` / `Delivery` | Si el usuario tenía un pedido activo, se devuelve; si no, se informa que puede comenzar uno nuevo. |
| `RequestNearbyRestaurants(ClientDTO)`   | `Client`                             | `Coordinator`                        | Solicita restaurantes cercanos.                                                                    |
| `RequestNearbyRestaurants(ClientDTO)`   | `Coordinator`                        | `NearbyRestaurantService`            | Solicita los restaurantes cercanos a un cliente.                                                   |
| `NearbyRestaurants(Vec<RestaurantDTO>)` | `NearbyRestaurantService`            | `Coordinator`                        | Respuesta con la lista de restaurantes.                                                            |
| `RequestThisOrder(OrderDTO)`            | `Client`                             | `Coordinator`                        | El cliente realiza un pedido.                                                                      |
| `AuthorizationResult(Result)`           | `Coordinator`                        | `Client`                             | Resultado de la autorización de pago.                                                              |
| `NotifyOrderUpdated(OrderDTO)`          | `Coordinator`                        | `Client`                             | Notifica actualizaciones en el estado del pedido.                                                  |
| `OrderFinalized(OrderDTO)`              | `Client`                             | `Coordinator`                        | El cliente indica que el pedido finalizó.                                                          |
| `NewOrder(OrderDTO)`                    | `Coordinator`                        | `Restaurant`                         | Envía un nuevo pedido al restaurante.                                                              |
| `CancelOrder(OrderDTO)`                 | `Restaurant`                         | `Coordinator`                        | El restaurante cancela el pedido.                                                                  |
| `UpdateOrderStatus(OrderDTO)`           | `Restaurant` / `Delivery`            | `Coordinator`                        | Informa el nuevo estado de un pedido.                                                              |

---

#### Gateway de Pagos

| Mensaje                          | Emisor           | Receptor         | Descripción                             |
| -------------------------------- | ---------------- | ---------------- | --------------------------------------- |
| `RequestAuthorization(OrderDTO)` | `Coordinator`    | `PaymentGateway` | Solicita autorización del pago.         |
| `RequestChargeOrder(OrderDTO)`   | `Coordinator`    | `PaymentGateway` | Solicita ejecutar el cobro.             |
| `AuthorizedOrder(OrderDTO)`      | `PaymentGateway` | `Coordinator`    | El pago fue autorizado.                 |
| `DeniedOrder(OrderDTO)`          | `PaymentGateway` | `Coordinator`    | El pago fue rechazado.                  |
| `SendMoney(OrderDTO)`            | `PaymentGateway` | `Coordinator`    | Se transfiere el dinero al restaurante. |

---

#### Asignación a Deliveries

| Mensaje                                  | Emisor                       | Receptor                   | Descripción                                     |
| ---------------------------------------- | ---------------------------- | -------------------------- | ----------------------------------------------- |
| `IAmAvailable(DeliveryDTO)`              | `Delivery`                   | `Coordinator`              | El delivery se declara disponible.              |
| `RequestNearbyDelivery(OrderDTO)`        | `Restaurant`                 | `Coordinator`              | Solicita deliveries disponibles para un pedido. |
| `RequestNearbyDeliveries(RestaurantDTO)` | `Coordinator`                | `NearbyDeliveryService`    | Pide deliveries cercanos.                       |
| `NearbyDeliveries(Vec<DeliveryDTO>)`     | `NearbyDeliveryService`      | `Coordinator`              | Entrega lista de deliveries cercanos.           |
| `DeliveryAvailable(OrderDTO)`            | `Coordinator`                | `Restaurant`               | Hay un delivery disponible para el pedido.      |
| `DeliverThisOrder(OrderDTO)`             | `Restaurant` / `Coordinator` | `Coordinator` / `Delivery` | Se envía el pedido para que sea entregado.      |
| `DeliveryNoNeeded(OrderDTO)`             | `Coordinator`                | `Delivery`                 | Informa que otro delivery fue asignado.         |
| `Delivered(OrderDTO)`                    | `Delivery`                   | `Coordinator`              | El delivery informa que completó la entrega.    |

---

#### Modificación del `Storage`

| Mensaje                                                        | Emisor                                        | Receptor  | Descripción                                    |
| -------------------------------------------------------------- | --------------------------------------------- | --------- | ---------------------------------------------- |
| `AddClient(ClientDTO)`                                         | `Coordinator` o cualquier servicio del server | `Storage` | Guarda un nuevo cliente                        |
| `AddRestaurant(RestaurantDTO)`                                 | `Coordinator` o cualquier servicio del server | `Storage` | Guarda un nuevo restaurante                    |
| `AddDelivery(DeliveryDTO)`                                     | `Coordinator` o cualquier servicio del server | `Storage` | Guarda un nuevo repartidor                     |
| `AddOrder(OrderDTO)`                                           | `Coordinator` o cualquier servicio del server | `Storage` | Guarda un nuevo pedido                         |
| `RemoveClient(client_id)`                                      | `Coordinator` o cualquier servicio del server | `Storage` | Elimina un cliente                             |
| `RemoveRestaurant(restaurant_id)`                              | `Coordinator` o cualquier servicio del server | `Storage` | Elimina un restaurante                         |
| `RemoveDelivery(delivery_id)`                                  | `Coordinator` o cualquier servicio del server | `Storage` | Elimina un repartidor                          |
| `RemoveOrder(order_id)`                                        | `Coordinator` o cualquier servicio del server | `Storage` | Elimina un nuevo pedido                        |
| `SetOrderToClient(client_id, order_id)`                        | `Coordinator` o cualquier servicio del server | `Storage` | Asocia una orden con un cliente                |
| `AddAuthorizedOrderToRestaurant(restaurant_id, order_id)`      | `Coordinator` o cualquier servicio del server | `Storage` | Agrega una orden `AUTHORIZED` al restaurante   |
| `AddPendingOrderToRestaurant(restauant_id, order_id)`          | `Coordinator` o cualquier servicio del server | `Storage` | Agrega una orden `PENDING` al restaurante      |
| `RemoveAuthorizedOrderFromRestaurant(restaurant_id, order_id)` | `Coordinator` o cualquier servicio del server | `Storage` | Elimina una orden `AUTHORIZED` del restaurante |
| `RemovePendingOrderFromRestaurant(restauant_id, order_id)`     | `Coordinator` o cualquier servicio del server | `Storage` | Elimina una orden `PENDING` del restaurante    |
| `AddAuthorizedOrderToRestaurant(restaurant_id, order_id)`      | `Coordinator` o cualquier servicio del server | `Storage` | Agrega una orden `AUTHORIZED` al restaurante   |
| `SetDeliveryPosition(delivery_id, (f32, f32))`                 | `Coordinator` o cualquier servicio del server | `Storage` | Guarda la posición del repartidor              |
| `SetCurrentClientToDelivery(delivery_id, client_id)`           | `Coordinator` o cualquier servicio del server | `Storage` | Guarda el cliente actual del repartidor        |
| `SetCurrentOrderToDelivery(delivery_id, order_id)`             | `Coordinator` o cualquier servicio del server | `Storage` | Guarda el pedido actual del repartidor         |
| `SetDeliveryStatus(delivery_id, DeliveryStatus)`               | `Coordinator` o cualquier servicio del server | `Storage` | Guarda el nuevo estado del repartidor          |
| `SetDeliveryToOrder(order_id, delivery_id)`                    | `Coordinator` o cualquier servicio del server | `Storage` | Guarda el repartidor asignado al pedido        |
| `SetOrderStatus(order_id, OrderStatus)`                        | `Coordinator` o cualquier servicio del server | `Storage` | Guarda el nuevo estado del pedido              |

---

### **Proceso `PaymentGateway`**

El proceso `PaymentGateway` simula un gateway de pagos que autoriza y cobra órdenes de pedido. Se ejecuta como un servicio independiente, escuchando conexiones de procesos `Coordinator`, y responde a solicitudes de autorización o cobro. Es responsable de validar pedidos y decidir si se aprueban, así como de efectuar el cobro de órdenes previamente autorizadas.

El proceso está compuesto por dos actores principales:

- [`Acceptor`](#paymentgateway-async)
- [`PaymentGateway`](#paymentgateway-async)

Además, contiene un [`Communicator`](#communicator-async) al igual que otros procesos.

<p align="center">
  <img src="img/paymentgateway_architecture.jpg" style="max-width: 100%; height: auto;" alt="Arquitectura del PaymentGateway">
</p>

#### Tabla de estados del pedido (desde la perspectiva del PaymentGateway)

| Estado Inicial     | Evento o Acción              | Estado Final | Actor Responsable | Comentario                                                 |
| ------------------ | ---------------------------- | ------------ | ----------------- | ---------------------------------------------------------- |
| `NO_RECORD`        | Llega `RequestAuthorization` | `AUTHORIZED` | `Communicator`    | Se autoriza la orden y se guarda en memoria.               |
| `NO_RECORD`        | Llega `RequestAuthorization` | `DENIED`     | `Communicator`    | Se rechaza la orden (probabilidad).                        |
| `AUTHORIZED`       | Llega `RequestChargeOrder`   | `CHARGED`    | `Communicator`    | Se efectúa el cobro de la orden previamente autorizada.    |
| `DENIED` o ausente | Llega `RequestChargeOrder`   | (Sin cambio) | `Communicator`    | La orden no existe o fue denegada, no se realiza el cobro. |

---

#### 💵 **PaymentGateway** _(Async)_

El actor **PaymentGateway** representa el servidor principal que escucha conexiones en el puerto 8080. Su función es aceptar conexiones de Coordinators, y delegar el manejo de cada conexión a un actor `Communicator`.

Responsabilidades:

- Iniciar el socket y aceptar conexiones TCP entrantes.
- Crear un `Communicator` para cada conexión.
- Mantener un diccionario de órdenes autorizadas (`order_id → OrderDTO`).

##### Estado interno de `PaymentGateway`

```rust
pub struct PaymentGateway {
    /// Conjunto de IDs de pedidos que han sido autorizados para el pago.
    pub authorized_orders: HashSet<u64>,
    /// Comunicadores activos mapeados por dirección remota.
    pub communicators: HashMap<SocketAddr, Communicator<PaymentGateway>>,
    /// Probabilidad de que un pedido sea autorizado (entre 0.0 y 1.0).
    pub probability_of_success: f32,
    /// Registrador de eventos del gateway de pagos.
    pub logger: Logger,
}

```

---

### **Proceso `Cliente`**

Cada proceso `Cliente` representa a un comensal dentro del sistema. Se ejecuta en una consola independiente y se comunica únicamente con un proceso `Server` mediante mensajes TCP. Su función principal es realizar pedidos, esperar su procesamiento, y recibir notificaciones del estado de su orden.

El proceso está compuesto por dos actores principales:

- [`UIHandler`](#uihandler-async)
- [`Client`](#client-async)

<p align="center">
  <img src="img/client_architecture.jpg" style="max-width: 100%; height: auto;" alt="Client Architecture">
</p>

#### Tabla de estados del pedido (desde la perspectiva del Cliente)

| Estado Inicial          | Evento o Acción                     | Estado Final         | Actor Responsable    | Comentario                                                          |
| ----------------------- | ----------------------------------- | -------------------- | -------------------- | ------------------------------------------------------------------- |
| `NONE`                  | Cliente realiza un pedido           | `REQUESTED`          | `UIHandler → Client` | El cliente elige restaurante y producto, y envía el pedido inicial. |
| `REQUESTED`             | Server responde con `AUTHORIZED`    | `AUTHORIZED`         | `Server → Client`    | El pedido fue autorizado por el `PaymentGateway`.                   |
| `REQUESTED`             | Server responde con `CANCELLED`     | `CANCELLED`          | `Server → Client`    | El pedido fue rechazado por el `PaymentGateway`.                    |
| `AUTHORIZED`            | Restaurante acepta el pedido        | `PENDING`            | `Server → Client`    | El restaurante acepta preparar el pedido.                           |
| `AUTHORIZED`            | Restaurante rechaza el pedido       | `CANCELLED`          | `Server → Client`    | El restaurante rechaza el pedido.                                   |
| `PENDING`               | Pedido asignado a chef              | `PREPARING`          | `Server → Client`    | El pedido comenzó a prepararse en la cocina.                        |
| `PREPARING`             | Cocina finaliza y pasa a reparto    | `READY_FOR_DELIVERY` | `Server → Client`    | El pedido está listo para ser despachado.                           |
| `READY_FOR_DELIVERY`    | Pedido asignado a un delivery       | `DELIVERING`         | `Server → Client`    | Un delivery fue asignado y está en camino.                          |
| `DELIVERING`            | Pedido entregado por el delivery    | `DELIVERED`          | `Server → Client`    | El cliente recibe el pedido.                                        |
| _Cualquiera intermedio_ | Pedido cancelado en cualquier etapa | `CANCELLED`          | `Server → Client`    | Por rechazo de restaurante, problema con delivery u otra razón.     |

---

#### 🎛️ **UIHandler** _(Async)_

El actor **UIHandler** representa la interfaz de interacción humano-sistema. Su rol es recolectar inputs del usuario y mostrar por pantalla información relevante que llega desde el sistema.

Responsabilidades:

- Leer inputs del usuario (nombre, pedido y elección de restaurante).
- Mostrar mensajes y estados del pedido.
- Comunicarse con el actor `Client` enviando mensajes.

##### Estado interno de `UIHandler`

```rust
pub struct UIHandler {
    /// Dirección del actor `Client` al que se envían las selecciones del usuario.
    pub client: Addr<Client>,
    /// Logger para mensajes y errores relacionados con la interfaz de usuario.
    pub logger: Logger,
}
```

---

#### 🙋🏻‍♂️ **Client** _(Async)_

El actor **Client** representa la lógica del comensal. Es el encargado de interactuar con el `Server`, tomar decisiones basadas en la información recibida, y mantener el estado interno del cliente.

Responsabilidades:

1. Conectarse al `Server` (descubrir quién es el coordinador).
2. Identificarse con su ID único.
3. Intentar recuperar su estado previo si hubo una desconexión (operación `RECOVER`).
4. Solicitar restaurantes cercanos a su ubicación.
5. Enviar la orden al restaurante elegido.
6. Esperar la aprobación del `PaymentGateway`.
7. Esperar actualizaciones del estado del pedido.
8. Finalizar cuando el pedido es recibido o cancelado.

##### Estado interno de `Client`

```rust
pub struct Client {
    /// Lista de direcciones de socket del servidor a las que conectarse.
    pub servers: Vec<SocketAddr>,
    /// Identificador único para el cliente.
    pub client_id: String,
    /// Posición actual del cliente en coordenadas 2D.
    pub client_position: (f32, f32),
    /// Pedido actual realizado por el cliente, si existe.
    pub client_order: Option<OrderDTO>,
    /// Dirección del actor que maneja la interfaz de usuario (UI).
    pub ui_handler: Option<Addr<UIHandler>>,
    /// Comunicador para las interacciones de red con el servidor.
    pub communicator: Option<Communicator<Client>>,
    /// Flujo TCP pendiente antes de que el actor inicie.
    pub pending_stream: Option<TcpStream>,
    /// Registrador de eventos del cliente.
    pub logger: Logger,
    /// Manejador para el temporizador de entrega, si está activo.
    delivery_timer: Option<actix::SpawnHandle>,
    /// Temporizador para esperar intentos de reconexión después de que una conexión se cierre.
    waiting_reconnection_timer: Option<actix::SpawnHandle>,
    /// Bandera para indicar si el cliente ya está conectado y esperando la reconexión.
    already_connected: bool,
}

```

---

### Mensajes del Proceso `Client`

| Mensaje                                              | Emisor        | Receptor                 | Descripción                                                                                                                                            |
| ---------------------------------------------------- | ------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `WhoIsLeader`                                        | `Client`      | `Todos los Coordinators` | Consulta inicial para saber quién es el coordinador actual del sistema.                                                                                |
| `LeaderIs(SocketAddr)`                               | `Coordinator` | `Client`                 | Respuesta con la dirección IP y puerto del coordinador líder                                                                                           |
| `RequestID`                                          | `Client`      | `UIHandler`              | Petición al usuario para que ingrese su identificador único.                                                                                           |
| `SendID(ClientID)`                                   | `UIHandler`   | `Client`                 | El usuario introduce su ID (`String`) y lo envía al actor `Client`.                                                                                    |
| `RegisterUser(ClientDTO)`                            | `Client`      | `Coordinator`            | Solicitud para intentar recuperar un pedido anterior en caso de haber sido desconectado.                                                               |
| `RecoveredInfo(Option<ClientDTO>, Option<OrderDTO>)` | `Coordinator` | `Client`                 | Si el cliente tenía un pedido activo, se devuelve `ClientDTO` y `OrderDTO` con su estado actual. Si no, se envía `None` para comenzar un nuevo pedido. |
| `RequestNearbyRestaurants(<ClientDTO>)`              | `Client`      | `Coordinator`            | Solicita al coordinador los restaurantes más cercanos según la ubicación del cliente.                                                                  |
| `NearbyRestaurants(Vec<RestaurantInfo>)`             | `Coordinator` | `Client`                 | Devuelve una lista de los `RestaurantInfo` (con su id y ubicación) restaurantes cercanos disponibles.                                                  |
| `SelectNearbyRestaurants(Vec<RestaurantInfo>)`       | `Client`      | `UIHandler`              | Instrucción al `UIHandler` para que le muestre al usuario la lista de restaurantes y permita elegir uno.                                               |
| `SendThisOrder(<OrderDTO>)`                          | `UIHandler`   | `Client`                 | El usuario completa el pedido (por ejemplo elige restaurante, tipo de comida, etc.) y lo envía al actor `Client`.                                      |
| `RequestThisOrder(<OrderDTO>)`                       | `Client`      | `Coordinator`            | Solicita al coordinador que autorice el pedido. Éste lo reenvía al `PaymentGateway`.                                                                   |
| `AuthorizationResult(Result)`                        | `Coordinator` | `Client`                 | Resultado de la autorización: `Ok` si fue aprobada, `Err` si fue rechazada por el `PaymentGateway`.                                                    |
| `NotifyOrderUpdated(<OrderDTO>)`                     | `Coordinator` | `Client`                 | Notificación de actualización del estado del pedido (ej. “en preparación”, “en camino”, etc.).                                                         |
| `OrderFinalized(<OrderDTO>)`                         | `Client`      | `Coordinator`            | Indica que el pedido fue completado (`Delivered`) o cancelado (`Cancelled`). El proceso del cliente finaliza.                                          |

---

#### **Proceso `Restaurante`** _(Async)_

El proceso `Restaurante` agrupa múltiples actores que simulan distintas funciones internas de un restaurante (recepción de pedidos, cocina, preparación, entrega). Es el encargado de procesar pedidos entrantes, gestionarlos a través de chefs y despacharlos mediante repartidores cercanos.

**Responsabilidades:**

1. Conectarse al `Server` y registrarse como restaurante disponible.
2. Intentar recuperar su estado previo si hubo una desconexión (operación `RECOVER`).
3. Recibir pedidos nuevos (en estado `PENDING` o `AUTHORIZED`) y redirigirlos correctamente.
4. Decidir si acepta o rechaza pedidos `AUTHORIZED`.
5. Gestionar una cola de pedidos para preparar.
6. Coordinar a los `Chef`s para cocinar pedidos.
7. Solicitar algún repartidor cercano al `Server` cuando un pedido esté listo.
8. Finalizar su participación en un pedido una vez que ha sido entregado o cancelado.

<p align="center">
  <img src="img/restaurant_architecture.jpg" style="max-width: 100%; height: auto;" alt="Restaurant Architecture">
</p>

#### Tabla de estados del pedido (desde la perspectiva del Restaurante)

| Estado Inicial       | Acción del Restaurante        | Estado Final         | Actor Responsable           | Comentario                                                                 |
| -------------------- | ----------------------------- | -------------------- | --------------------------- | -------------------------------------------------------------------------- |
| `PENDING`            | Pedido recibido y encolado    | `PENDING`            | `Restaurant → Kitchen`      | Pasa directo a cocina.                                                     |
| `AUTHORIZED`         | Restaurante lo rechaza        | `CANCELLED`          | `Restaurant`                | Se envía `CancelOrder` al `Server`.                                        |
| `AUTHORIZED`         | Restaurante lo acepta         | `PENDING`            | `Restaurant → Kitchen`      | Se informa al `Server` (y este al `Client`) que fue aceptado.              |
| `PENDING`            | Pedido asignado a chef        | `PREPARING`          | `Kitchen → Server`          | Se informa al `Server` (y este al `Client`) que comenzó la preparación.    |
| `PREPARING`          | Chef termina la cocción       | `READY_FOR_DELIVERY` | `Chef → DeliveryAssigner`   | Se informa al `Server` (y este al `Client`) que está listo para despachar. |
| `READY_FOR_DELIVERY` | Pedido asignado a un delivery | `DELIVERING`         | `DeliveryAssigner → Server` | Se notifica al `Server` (y este al `Client`) con `DeliverThisOrder`.       |

---

#### 🍽 **Restaurant** _(Async)_

Encargado de recibir pedidos provenientes del `Server` y reenviarlos al componente adecuado según su estado (`PENDING` o `AUTHORIZED`).

**Responsabilidades:**

- Conectarse al `Server` y realizar el proceso de `Recover`.
- Recibir nuevos pedidos desde el `Server`.
- Enviar directamente a `Kitchen` los pedidos `PENDING`.
- Para pedidos `AUTHORIZED`:

  - Confirmar (enviar a `Kitchen` + `UpdateOrderStatus(Pending)` al `Server`).
  - O rechazar (`CancelOrder` al `Server`).

##### Estado interno de `Restaurant`

```rust
pub struct Restaurant {
    /// Información básica sobre el restaurante.
    pub info: RestaurantInfo,
    /// Probabilidad de aceptar o rechazar un pedido.
    pub probability: f32,
    /// Dirección del actor de la cocina.
    pub kitchen_address: Option<Addr<Kitchen>>,
    /// Dirección del actor asignador de entregas.
    pub delivery_assigner_address: Option<Addr<DeliveryAssigner>>,
    /// Comunicador de red para la interacción con el servidor.
    pub communicator: Option<Communicator<Restaurant>>,
    /// Flujo TCP pendiente antes de que el actor comience.
    pub pending_stream: Option<TcpStream>,
    /// Registrador de eventos del restaurante.
    pub logger: Logger,
    /// Lista de direcciones de socket de los servidores.
    pub servers: Vec<SocketAddr>,
    /// Manejador del temporizador para esperar intentos de reconexión.
    waiting_reconnection_timer: Option<actix::SpawnHandle>,
    /// Bandera que indica si el restaurante ya está conectado a un servidor.
    pub already_connected: bool,
}
```

---

#### 🍳 **Kitchen** _(Async)_

Gestiona la cola de pedidos que deben prepararse y coordina a los chefs disponibles.

**Responsabilidades:**

- Mantener la cola de pedidos en espera.
- Asignar pedidos a chefs disponibles.
- Informar al `Server` cuando un pedido entra en estado `Preparing`.

##### Estado interno de `Kitchen`

```rust
pub struct Kitchen {
    /// Cola de pedidos esperando ser preparados.
    pub pending_orders: VecDeque<OrderDTO>,
    /// Cola de chefs disponibles.
    pub chefs_available: VecDeque<Addr<Chef>>,
    /// Dirección del actor del restaurante principal.
    pub my_restaurant: Addr<Restaurant>,
    /// Dirección del actor asignador de entregas.
    pub my_delivery_assigner: Addr<DeliveryAssigner>,
    /// Registrador de eventos de la cocina.
    pub logger: Logger,
}

```

---

#### 🧑‍🍳 **Chef** _(Async)_

Simula la preparación de un pedido, demora un tiempo artificial y notifica cuando el pedido está listo para ser despachado.

**Responsabilidades:**

- Cocinar los pedidos asignados (delay simulado).
- Notificar al `DeliveryAssigner` con `SendThisOrder`.
- Avisar a la `Kitchen` que está disponible nuevamente (`IAmAvailable`).

##### Estado interno de `Chef`

```rust
pub struct Chef {
    /// Tiempo estimado para cocinar un pedido.
    pub time_to_cook: Duration,
    /// El pedido que se está preparando actualmente.
    pub order: Option<OrderDTO>,
    /// Dirección del actor asignador de entregas.
    pub delivery_assigner_address: Addr<DeliveryAssigner>,
    /// Dirección del actor de la cocina.
    pub kitchen_address: Addr<Kitchen>,
    /// Registrador de eventos del chef.
    pub logger: Logger,
}
```

---

#### 🔎 **DeliveryAssigner** _(Async)_

Encargado de pedir repartidores al `Server` y asociarlos con pedidos listos para entregar.

**Responsabilidades:**

- Encolar pedidos listos para despacho.
- Solicitar deliverys al `Server`.
- Manejar llegadas de `DeliveryAvailable`.
- Enviar `DeliverThisOrder` al `Server`.

##### Estado interno de `DeliveryAssigner`

```rust
pub struct DeliveryAssigner {
    /// Información sobre el restaurante.
    pub restaurant_info: RestaurantInfo,
    /// Pedidos listos para ser despachados.
    pub ready_orders: HashMap<u64, OrderDTO>,
    /// Mapeo de pedidos a IDs de entregas asignadas.
    pub orders_delivery: HashMap<u64, String>,
    /// Dirección del actor del restaurante principal.
    pub my_restaurant: Addr<Restaurant>,
    /// Registrador de eventos del asignador de entregas.
    pub logger: Logger,
}
```

---

### Mensajes del Proceso `Restaurant`

| Mensaje                                     | Emisor             | Receptor           | Descripción                                                                                                  |
| ------------------------------------------- | ------------------ | ------------------ | ------------------------------------------------------------------------------------------------------------ |
| `RegisterUser(RestaurantDTO)`               | `Restaurant`       | `Coordinator`      | Mensaje inicial de registro del restaurante en el sistema y recuperación de datos (en caso de ser necesario) |
| `RecoveredInfo(Option<RestaurantDTO>)`      | `Coordinator`      | `Restaurant`       | Si el ya estaba registrado, se devuelve `RestaurantDTO` con su estado actual. Si no, se envía `None`.        |
| `NewOrder(OrderDTO)`                        | `Coordinator`      | `Restaurant`       | Llega un nuevo pedido al restaurante. Puede estar en estado `PENDING` o `AUTHORIZED`.                        |
| `SendToKitchen(OrderDTO)`                   | `Restaurant`       | `Kitchen`          | Pedido `PENDING` enviado a la cocina.                                                                        |
| `CancelOrder(OrderDTO)`                     | `Restaurant`       | `Coordinator`      | El restaurante rechaza un pedido `AUTHORIZED`. Se informa al servidor para que lo cancele.                   |
| `UpdateOrderStatus(OrderDTO)`               | `Restaurant`       | `Coordinator`      | El restaurante acepta un pedido `AUTHORIZED`. Se informa al `Coordinator` (y al `Client`).                   |
| `AssignToChef(Order)`                       | `Kitchen`          | `Chef`             | La cocina asigna un pedido a un chef disponible.                                                             |
| `OrderIsPreparing(OrderDTO)`                | `Kitchen`          | `Coordinator`      | Se informa al `Coordinator` (y al `Client`) que un pedido ha comenzado su preparación.                       |
| `SendThisOrder(Order)`                      | `Chef`             | `DeliveryAssigner` | El chef terminó la preparación y pasa el pedido al despachador.                                              |
| `IAmAvailable(Addr<Chef>)`                  | `Chef`             | `Kitchen`          | El chef se libera y notifica a la cocina que puede recibir otro pedido.                                      |
| `RequestDelivery(OrderDTO, RestaurantInfo)` | `DeliveryAssigner` | `Coordinator`      | Solicita al `Coordinator` un delivery cercano para el pedido listo.                                          |
| `DeliveryAvailable(OrderDTO)`               | `Coordinator`      | `DeliveryAssigner` | Llega un delivery disponible para un pedido.                                                                 |
| `DeliverThisOrder(OrderDTO)`                | `DeliveryAssigner` | `Coordinator`      | Se asocia el pedido con un delivery y se envía al `Coordinator` (y este al `Client`).                        |

---

#### **Proceso `Delivery`** _(Async)_

El proceso `Delivery` representa a un repartidor autónomo. Su función es aceptar y realizar entregas de pedidos que ya han sido preparados por un restaurante, coordinándose con el `Server` para recibir asignaciones y reportar finalizaciones. Puede desconectarse y reconectarse, intentando recuperar su estado anterior en caso de haber estado en medio de una entrega.

**Responsabilidades:**

1. Inicializarse con un nombre único y su ubicación actual por línea de comandos.
2. Descubrir y conectarse con el `Server` (coordinador actual).
3. Registrarse como disponible para hacer entregas (`IAmAvailable`).
4. Intentar recuperar su estado anterior en caso de una reconexión (`Recover`).
5. Recibir ofertas de entrega (`NewOfferToDeliver`) y decidir si aceptarlas.
6. En caso de aceptar una oferta, esperar la confirmación (`DeliverThisOrder`) para iniciar el reparto.
7. Simular el viaje y notificar al `Server` con `Delivered`.
8. Repetir el ciclo o desconectarse temporalmente según preferencia.

<p align="center">
  <img src="img/delivery_architecture.jpg" style="max-width: 100%; height: auto;" alt="Delivery Architecture">
</p>

#### Tabla de estados del Delivery

| Estado Actual         | Evento o Acción                     | Nuevo Estado          | Acción del Delivery                        | Comentario                                                                 |
| --------------------- | ----------------------------------- | --------------------- | ------------------------------------------ | -------------------------------------------------------------------------- |
| `INITIAL`             | Se lanza el proceso                 | `RECONNECTING`        | Establece conexión con `Server`            | Comienza el descubrimiento de coordinador (`who is coord?`).               |
| `RECONNECTING`        | Se conecta al `Server`              | `RECOVERING`          | Enviar `Recover(delivery_id)`              | Informa su `delivery_id` y solicita estado previo.                         |
| `RECOVERING`          | Respuesta con datos de entrega      | `DELIVERING`          | Reanuda entrega pendiente                  | Retoma un pedido que había quedado en curso.                               |
| `RECOVERING`          | Respuesta sin datos                 | `AVAILABLE`           | Enviar `IAmAvailable(delivery_id, pos)`    | No estaba entregando, se registra como disponible.                         |
| `AVAILABLE`           | Recibe `NewOfferToDeliver`          | `WAITINGCONFIRMATION` | Si acepta: enviar `DeliveryAccepted(order)`   | Si no acepta, ignora el mensaje y sigue disponible.                        |
| `WAITINGCONFIRMATION` | Recibe `DeliveryNoNeeded`           | `AVAILABLE`           | Espera o decide reconectarse más adelante  | Otro delivery fue asignado más rápido.                                     |
| `WAITINGCONFIRMATION` | Recibe `DeliverThisOrder`           | `DELIVERING`          | Inicia simulación de entrega               | Confirmación final de asignación del pedido.                               |
| `DELIVERING`          | Termina la entrega (viaje simulado) | `AVAILABLE`           | Enviar `Delivered(order)` + `IAmAvailable` | Informa finalización y vuelve a estar disponible para nuevas asignaciones. |

---

#### 🛵 **Delivery** _(Async)_

El actor `Delivery` encapsula toda la lógica de un repartidor. Mantiene su estado interno (ubicación, ocupación actual, pedido activo si lo hubiera) y se comunica exclusivamente con el `Server`.

**Responsabilidades:**

- Realizar el proceso de `Recover` para detectar si tiene un pedido en curso.
- Reportar disponibilidad al `Server`.
- Evaluar ofertas de entrega y responder si está libre.
- Ejecutar la entrega una vez confirmada por el `Server`.
- Simular el tiempo de viaje y finalizar el pedido.

##### Estado interno de `Delivery`

```rust
pub struct Delivery {
  /// Lista de direcciones de los servidores a los que puede conectarse.
  pub servers: Vec<SocketAddr>,
  /// Identificador único del delivery.
  pub delivery_id: String,
  /// Posición actual del delivery.
  pub position: (f32, f32),
  /// Estado actual del delivery: Disponible, Ocupado, Entregando, etc.
  pub status: DeliveryStatus,
  /// Probabilidad de rechazar un pedido disponible.
  pub probability: f32,
  /// Pedido actual en curso, si lo hay.
  pub current_order: Option<OrderDTO>,
  /// Comunicador para la interacción en red con el servidor.
  pub communicator: Option<Communicator<Delivery>>,
  /// Stream TCP pendiente antes de que el actor inicie.
  pub pending_stream: Option<TcpStream>,
  /// Logger para eventos del delivery.
  pub logger: Logger,
  /// Timer para manejar intentos de reconexión.
  waiting_reconnection_timer: Option<actix::SpawnHandle>,
  /// Timer para mantener vivo el actor durante reconexiones.
  keep_alive_timer: Option<actix::SpawnHandle>,
  /// Flag que indica si el delivery ya está conectado y esperando reconexión.
  already_connected: bool,
}
```

##### Estados del `DeliveryStatus`

```rust
pub enum DeliveryStatus {
    /// Reconectando a un servidor
    Reconnecting,
    /// Recuperando el estado de un pedido
    Recovering,
    /// Disponible para aceptar pedidos
    Available,
    /// Esperando la confirmación del restaurante después de aceptar un pedido
    WaitingConfirmation,
    /// Entregando un pedido
    Delivering,
}

```

---

### Mensajes del Proceso `Delivery`

| Mensaje                                   | Emisor        | Receptor                       | Descripción                                                                   |
| ----------------------------------------- | ------------- | ------------------------------ | ----------------------------------------------------------------------------- |
| `WhoIsLeader`                             | `Delivery`    | Todos los `CoordinatorManager` | Consulta inicial para determinar quién es el coordinador actual del sistema.  |
| `RegisterUser(DeliveryDTO)`               | `Delivery`    | `Coordinator`                  | Registro del delivery como nodo activo.                                       |
| `RecoveredUserInfo(Option<DeliveryDTO>)`  | `Coordinator` | Delivery                       | Respuesta con los datos del delivery si estaba activo antes de desconectarse. |
| `IAmAvailable(DeliveryDTO)`               | `Delivery`    | `Coordinator`                  | Informa que está disponible para realizar entregas.                           |
| `NewOfferToDeliver(DeliveryID, OrderDTO)` | `Coordinator` | `Delivery`                     | Oferta de un nuevo pedido para entregar.                                      |
| `DeliveryAccepted(OrderDTO)`                 | `Delivery`    | `Coordinator`                  | El delivery acepta el pedido y pasa a estado ocupado.                         |
| `DeliveryNoNeeded(OrderDTO)`              | `Coordinator` | `Delivery`                     | Notificación de que el pedido fue asignado a otro delivery (descarta oferta). |
| `DeliverThisOrder(OrderDTO)`              | `Coordinator` | `Delivery`                     | Confirmación definitiva de que debe entregar el pedido.                       |
| `Delivered(OrderDTO)`                     | `Delivery`    | `Coordinator`                  | Notifica que finalizó la entrega.                                             |

---

### Modelo de replicación de servidores y tolerancia a fallos

La resiliencia del sistema se garantiza mediante la ejecución simultánea de **múltiples instancias del proceso servidor**, conectadas entre sí formando una **topología de anillo lógico**. Esta estrategia permite asegurar **alta disponibilidad**, **tolerancia a fallos** y **consistencia eventual** ante caídas o reinicios de alguna de las instancias.

#### Topología en anillo

Las instancias del servidor se organizan siguiendo un **orden total** definido por la IP y el puerto:

- Primero se ordenan por dirección IP (en forma creciente).
- En caso de IPs iguales, se desempata por el número de puerto (también en forma creciente).

Cada instancia mantiene **dos conexiones TCP activas**:

- Una con su **vecino anterior** en el anillo.
- Otra con su **vecino siguiente**.

Esto reduce el número total de conexiones abiertas y simplifica la lógica de comunicación.

<p align="center">
  <img src="img/ring_topology.jpg" style="max-width: 100%; height: auto;" alt="Topología en anillo del servidor">
</p>

---

#### Inicio de una nueva instancia

Cuando una nueva instancia del servidor se inicia:

1. **Carga la lista de IPs y puertos** de todas las posibles instancias del sistema.
2. Intenta establecer conexión con el resto.
3. A cada servidor conectado le envía un mensaje `WhoIsLeader`.
4. Los servidores que respondan le devuelven el mensaje `LeaderIs` con la identidad del líder actual.
5. Una vez identificado el líder, la nueva instancia se considera un **servidor secundario** y se integra al anillo.

> **Si no obtiene respuesta de nadie**, se considera la única instancia activa y se **autoproclama líder**, enviando el mensaje `IAmLeader`.

<p align="center">
  <img src="img/start_server.jpg" style="max-width: 100%; height: auto;" alt="Inicio del servidor">
</p>

---

#### Replicación del estado y los datos almacenados

Mientras un coordinador se encuentra activo como líder, todos los clientes, restaurantes y repartidores intercambian mensajes con él, actualizando y consultando su estado interno de manera indirecta. Las otras instancias del servidor no atienden estas solicitudes, por lo que implementan un mecanismo de actualización entre ellas y el líder para poder mantener la misma información.

El mecanismo basa su funcionamiento en el **registro de cambios del `Storage`**. Cada servidor mantiene un diccionario con todas las altas/bajas/modificaciones de los datos almacenados. Cada elemento del diccionario consiste en un índice (numérico), el cual indica un orden cronológico entre operaciones. Los valores son los **mensajes** recibidos por el `Storage`. Cuando una instancia (no líder) desea actualizar su registro de cambios para estar al día, se realizan los siguientes pasos:

1. La instancia envía el mensaje `RequestNewStorageUpdates` a la instancia anterior del anillo (la que es inmediatamente menor). El mensaje contiene el índice de la primera operación que posee en su registro.

2. La instancia anterior a la que desea actualizarse busca todas las operaciones realizadas cuyo índice se encuentre entre el índice recibido y la última disponible, y con ellas le responde a la instancia original mediante un mensaje `StorageUpdates`.

3. La instancia original recibe el mensaje, y con su payload arma y le envía el mensaje `ApplyStorageUpdates` a su `Storage`. Este último maneja el mensaje de diferente forma si la instancia que desea actualizarse es el líder o no:

- Si **es el líder**, elimina directamente todas las operaciones recibidas de su registro, ya que está seguro que esas operaciones dieron toda la vuelta al anillo y todas las instancias ya las registraron.

- Si **no es el líder**, aplica (y guarda en su registro) algunas de las operaciones recibidas, a la vez que borra otras operaciones de su registro. Para entender bien la lógica en este caso, planteemos un ejemplo. Supongamos que el `Storage` de la instancia posee en su registro las operaciones `1, 2, 3, 4`, y recibe el mensaje `ApplyStorageUpdates` con las operaciones `2, 3, 4, 5, 6`. El `Storage` divide a todas estas operaciones en tres grupos:
  - Operaciones que están en el registro pero NO en el mensaje recibido (`1`): esto quiere decir que la instancia anterior del anillo sabe que esas operaciones ya fueron registradas por todas las instancias y por eso las eliminó. Sabiendo esto, la instancia actual **las elimina de su registro**.
  - Operaciones que están tanto en el registro como en el mensaje recibido (`2, 3, 4`): estas operaciones ya fueron aplicadas previamente, **no se hace nada con ellas**.
  - Operaciones que están en el mensaje recibido pero NO en el registro (`5, 6`): estas son operaciones nuevas para la instancia actual, por lo que **las aplica y las guarda en su registro**.

El siguiente diagrama ilustra el funcionamiento del mecanismo de actualización:

<p align="center">
  <img src="img/storage_update.jpg" style="max-width: 100%; height: auto;" alt="Mecanismo de actualización del storage entre instancias del servidor">
</p>

Cabe destacar que cada instancia solicita actualizaciones cada cierto tiempo determinado. Así, los mensajes `RequestNewStorageUpdates` cumplen un doble propósito: además de mantener actualizados los datos, sirve de **ping** entre instancias para **detectar instancias caídas**.

¿Y qué sucede cuando una se inicia una nueva instancia, la cual debe solicitar **todo** el contenido del storage actual? En este caso, le envía el mensaje `RequestAllStorage` a la instancia anterior en el anillo. Esta última le responde con un mensaje `RecoverStorageOperations`, el cual contiene:

- Un diccionario de las operaciones necesarias para reconstruir el estado actual del `Storage`. La instancia con la información armó previamente este diccionario de mensajes recorriendo todo el contenido de su `Storage`. La nueva instancia le envía el mensaje `ApplyStorageUpdates` con estos cambios a su `Storage` para poder recuperar el estado.

- El registro de operaciones completo actual. La nueva instancia necesita conocer adicionalmente el registro para poder satisfacer solicitudes de actualización de su siguiente instancia en el anillo, como así también saber a partir de qué número de operación va a solicitar actualizaciones a partir de ese momento. Para ello, se le envía el mensaje `SetStorageUpdatesLog` (el cual contiene el registro de operaciones completo actual) al `Storage` para que este último guarde sus operaciones sin aplicarlas.

---

### Elección de líder

El sistema implementa un **algoritmo de elección distribuido** basado en la **topología en anillo**, conocido como **algoritmo de elección en anillo**. Este protocolo garantiza que, ante la caída del líder actual, se designe un nuevo líder de forma automática, única y sin necesidad de coordinación externa.

#### Detección de la caída del líder

La detección de la caída se realiza de forma descentralizada. Cada instancia del anillo consulta el estado de la instancia anterior a intervalos regulares mediante el mensaje `RequestStorageUpdates`. Si no obtiene respuesta dentro de un tiempo predefinido, o si la conexión TCP se pierde, la instancia infiere que su anterior ha caído. Si la instancia anterior era el líder, se inicia el proceso de elección de un nuevo líder.

#### Proceso de elección

Ante esta detección, se inicia el proceso de elección siguiendo estos pasos:

1. **Inicio de elección:**
   La instancia que detectó la falla envía el mensaje `LeaderElection` a su **siguiente nodo en el anillo**. Este mensaje incluye un vector que contiene su propia `SocketAddr` (dirección IP + puerto), identificador único de la instancia.

2. **Propagación del mensaje:**
   A medida que el mensaje circula por el anillo, cada instancia:

   - Verifica si ya participó en esta elección (si su `SocketAddr` está en el vector).
   - Si no participó, agrega su `SocketAddr` al vector y reenvía el mensaje al siguiente nodo.

3. **Determinación del nuevo líder:**
   Cuando el mensaje vuelve a una instancia cuyo `SocketAddr` ya estaba en el vector, significa que ha completado un ciclo completo. En ese momento, se elige como nuevo líder a la instancia cuya `SocketAddr` tenga el menor valor (según el orden natural IP\:puerto).

4. **Propagación del resultado:**
   La nueva instancia líder envía un mensaje `LeaderIs` al resto de las instancias, anunciando su elección.

#### Ejemplo visual

A continuación se ilustra un ejemplo simplificado del proceso, utilizando identificadores numéricos en lugar de direcciones `SocketAddr`:

<p align="center">
  <img src="img/leader_election_0.jpg" style="max-width: 100%; height: auto;" alt="Caída del líder">
</p>

<p align="center">
  <img src="img/leader_election_1.jpg" style="max-width: 100%; height: auto;" alt="Propagación del mensaje de elección">
</p>

<p align="center">
  <img src="img/leader_election_2.jpg" style="max-width: 100%; height: auto;" alt="Determinación del nuevo líder">
</p>

#### Caídas no críticas

Si la instancia que falla **no es el líder**, el sistema **no inicia una elección de líder**. En cambio, la instancia que detecta la desconexión simplemente restablece el anillo **reconectándose con el vecino anterior** del nodo caído, asegurando la continuidad de la topología.

#### Reincorporación de instancias

Cuando una instancia previamente caída se reincorpora, ejecuta el proceso de [inicio de una nueva instancia](#inicio-del-servidor), lo cual incluye:

- Recolección del estado actual del sistema.
- Reconexión con el anillo.
- Actualización de su rol (líder o seguidor).

Este mecanismo garantiza que el sistema se mantenga **coherente, resiliente y auto-recuperable** frente a fallos parciales.

## Cambios en la entrega final

### Heartbeats

La implementación final incluye un sistema donde los `CoordinatorManager` envían mensajes [`Ping`] al `CoordinatorManager` Líder. Este último responde con un mensaje [`Pong`]. Pasado un timeout de no recibir mensaje del Lider, se realiza una nueva elección de lider (usando el mensaje [`LeaderElection`]).

### Nuevos Mensajes implementados

- **Mensajes para conexiones**
  - `ConnectionClosed` : Notifica que una conexión TCP se cerró
  - `RegisterConnectionWithCoordinator` : Usado por el `Aceptor` para registrar una nueva conexión con un `CoordinadorManager`
  - `Ping` :Mensaje de heartbeat enviado periódicamente para verificar que la conexión sigue activa.
  - `Pong` : Respuesta al mensaje `Ping`, confirmando que la conexión está viva.
  - `CheckPongTimeout` : : Evento que verifica si se recibió un `Pong` dentro del tiempo esperado; si no, puede indicar una conexión caída.

- **Mensajes para el `Storage`**
  - `DeliveryExpectedTime`: Mensaje utilizado para informar al cliente sobre el tiempo estimado de entrega de su pedido. Permite que el cliente conozca cuánto falta para recibir su orden.
  - `OrderDelivered`: Mensaje enviado por el Delivery al servidor una vez que ha finalizado la entrega de un pedido. Permite actualizar el estado de la orden y notificar a las partes involucradas.
  - `IAmDelivering`: Mensaje enviado por el Delivery al servidor para notificar que está en proceso de entregar un pedido. Sirve para actualizar el estado del delivery y de la orden correspondiente.
  - `GetClient`: Permite consultar la información completa de un cliente específico a partir de su `client_id`. Devuelve un objeto `ClientDTO` con todos los datos asociados al cliente, o `None` si no existe.
  - `GetRestaurant` : Permite obtener la información de un restaurante específico usando su `restaurant_id`. Devuelve un objeto `RestaurantDTO` con los datos del restaurante, o `None` si no existe.
  - `GetDelivery` : Permite consultar la información de un repartidor (`delivery`) específico a partir de su `delivery_id`. Devuelve un objeto `DeliveryDTO` con los datos del delivery, o `None` si no existe.
  - `GetRestaurants` : Devuelve una lista con todos los restaurantes registrados en el sistema, incluyendo su información completa (`Vec<RestaurantDTO>`).
  - `GetAllRestaurantsInfo` : Devuelve una lista con la información básica (ID y posición) de todos los restaurantes, útil para mostrar opciones cercanas a un cliente (`Vec<RestaurantInfo>`).
  - `GetDeliveries` : Devuelve una lista con todos los repartidores registrados en el sistema, incluyendo su información completa (`Vec<DeliveryDTO>`).
  - `GetAllAvailableDeliveries` : Devuelve una lista con todos los repartidores que actualmente están disponibles para tomar pedidos, es decir, aquellos cuyo estado es `Available` (`Vec<DeliveryDTO>`).
  - `SetOrderExpectedTime`: Permite establecer en el storage el tiempo estimado de entrega para una orden específica. Este dato puede ser consultado por el cliente para saber cuánto demorará su pedido.
  - `InsertAcceptedDelivery`: Registra en el storage que un determinado delivery ha aceptado hacerse cargo de una orden. Se utiliza para evitar que otros deliveries acepten la misma orden y para llevar el control de asignaciones.
  - `RemoveAcceptedDeliveries`: Elimina del storage todas las ofertas de deliveries que ya han sido aceptadas para una orden, dejando solo la asignación definitiva. Esto asegura que no haya asignaciones duplicadas o conflictos.
  - `AddOrderAccepted`: Mensaje que indica que un delivery ha aceptado una oferta para entregar un pedido. Permite al storage registrar la intención del delivery antes de la confirmación final.
  - `FinishDeliveryAssignment`: Notifica al storage que una orden fue finalmente asignada a un delivery específico. Actualiza los estados correspondientes y limpia las ofertas pendientes para esa orden.
  - `GetMinLogIndex`: Solicita el índice mínimo del log de operaciones del storage. Es útil para sincronización y replicación entre servidores, permitiendo saber desde dónde deben solicitarse actualizaciones.
  - `GetLogsFromIndex`: Permite obtener todos los logs de operaciones del storage a partir de un índice dado. Se utiliza para replicar cambios y mantener la consistencia entre instancias del servidor.
  - `GetAllStorage`: Solicita una `Snapshot` completa del estado actual del storage, incluyendo clientes, restaurantes, deliveries y órdenes. Es fundamental para la recuperación de nodos nuevos o reiniciados.
  - `StorageSnapshot`: Mensaje que contiene una copia completa del estado actual del storage (clientes, restaurantes, deliveries, órdenes, etc). Se utiliza principalmente cuando un nuevo servidor se conecta y necesita sincronizar su estado con el resto del sistema.

- **Mensajes para el `PaymentGateway`**
  - `PaymentCompleted` : Usado para que el `PaymentGateway` comunique a las partes que el pago fue completado.
  - `BillPayment` : Usado por el `PaymentGateway` para pedirle al cliente el pago por un pedido.

- **Mensajes para el `Coordinator`**
  - `LeaderIdIs` : mensaje que informa a nodo sobre el Id del Líder actual.
  - `SetCoordinatorManager` : Mensaje para actualizar la referencia al `CoordinatorManager`
  - `SetActorsAddresses` : Actualiza las direcciones de los coordinadores y storage en el orderService.

- **Mensajes genéricos para los usuarios: `Client`, `Restaurant`, `Delivery`**
  - `Stop`
  - `StartRunning`
  - `RecoverProcedure` : Mensaje enviado para pedir la devolución de información de un usuario pos-crash.
  - `RetryLater` : Mensaje enviado pidiendo que se vuelva a reenviar el mensaje más tarde.
  - `Shutdown`: Señal para iniciar el proceso de cierre ordenado de la aplicación o componente.





## Instalación y Ejecución

### **Requisitos previos**

- [Rust (stable)](https://www.rust-lang.org/tools/install)
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
- Linux (recomendado para ejecución y pruebas)

### **Clonar el repositorio**

```bash
git clone https://github.com/iankvdh/PedidosRust.git
cd PedidosRust
```

### **Compilar el proyecto**

```bash
cargo build
```

Esto generará los binarios en `target/`.

### **Ejecución de los procesos**

Cada proceso debe ejecutarse en una terminal diferente. A continuación se muestra cómo lanzar los procesos principales:

#### **1. Lanzar el PaymentGateway**

```bash
cargo run --bin payment
```

#### **2. Lanzar los servidores**

En diferentes terminales, ejecutar:

```bash
cargo run --bin server 8081
cargo run --bin server 8082
cargo run --bin server 8083
cargo run --bin server 8084
```

#### **3. Lanzar clientes, restaurantes y deliveries**

Cada uno en una terminal distinta, por ejemplo:

```bash
cargo run --bin client cliente_1
cargo run --bin restaurant resto_1
cargo run --bin delivery delivery_1
```

En este ejemplo, `cliente_1`, `resto_1` y `delivery_1` son identificadores únicos para cada entidad.

> **Nota:** El nombre de usuario (`cliente_1`, `resto_1`, `delivery_1`, etc.) debe ser único en todo el sistema, incluso entre diferentes tipos de usuarios (clientes, restaurantes y deliveries). No puede haber dos entidades con el mismo nombre, sin importar su rol.

---

## Ejemplo de Ejecución

A continuación se muestra un flujo típico de ejecución del sistema:

1. **Iniciar el PaymentGateway y los servidores** (ver sección anterior).

2. **Iniciar un restaurante**:
   - El restaurante se conecta al servidor lider, recupera su estado en caso de existir y espera pedidos.
   - Recibe el pedido del cliente, lo acepta y lo pasa a cocina. Al terminarlo , notifica al asignador de repartidores que el pedido está listo para ser entregado.

3. **Iniciar un delivery**:
   - El delivery se conecta al servidor lider, recupera su estado en caso de existir o bien se declara disponible.
   - Cuando el pedido está listo, el servidor le ofrece la entrega.
   - El delivery acepta o rechaza. En caso de aceptar, realiza la entrega y notifica la finalización.

4. **Iniciar un cliente**:
   - El cliente se conecta al servidor líder.
   - Solicita restaurantes cercanos y selecciona uno.
   - Realiza un pedido.

5. **El cliente recibe notificaciones** del estado de su pedido en cada etapa.

Durante la ejecución, cada proceso mostrará logs informativos sobre los mensajes enviados y recibidos, así como el estado de los pedidos.

---

## Pruebas

### **Pruebas manuales**

Se recomienda probar los siguientes escenarios manualmente:

- **Flujo completo de pedido:** Cliente realiza un pedido, restaurante lo acepta, delivery lo entrega.
- **Tolerancia a fallos:** Detener y reiniciar un servidor, verificar que el sistema elige un nuevo líder y mantiene la consistencia.
- **Desconexión y reconexión de nodos:** Simular la caída y reconexión de clientes, restaurantes y deliveries.
- **Rechazo de pedidos:** Simular que el restaurante o el PaymentGateway rechazan un pedido.


### **Pruebas automáticas y de volumen**

Además de las pruebas manuales, el repositorio incluye **scripts de automatización** que permiten lanzar múltiples instancias de clientes, restaurantes, deliveries y servidores de forma simultánea. Estos scripts están diseñados para facilitar pruebas de **volumen**, **concurrencia** y **resiliencia** del sistema bajo diferentes escenarios.

- Los scripts permiten simular decenas de usuarios interactuando en paralelo, generando pedidos, aceptando/rechazando entregas y realizando reconexiones automáticas.
- Se pueden lanzar varios servidores a la vez para probar la tolerancia a fallos y la correcta replicación del estado.
- Los logs generados por cada proceso permiten analizar el comportamiento del sistema ante cargas elevadas y detectar posibles cuellos de botella o errores de sincronización.

> **Nota:** Consulta la carpeta `scripts/` del repositorio para ver ejemplos de uso y las instrucciones detalladas para ejecutar pruebas automáticas y de stress.

---

