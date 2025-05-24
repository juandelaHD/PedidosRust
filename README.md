<p align="center">
  <img src=img/logo_pedidos_rust.png width="300" alt="Logo PedidosRust">
</p>

# Programacion Concurrente - 2C2025 - PedidosRust

[![Review Assignment Due Date](https://classroom.github.com/assets/deadline-readme-button-22041afd0340ce965d47ae6ef1cefeee28c7c493a6346c4f15d667ab976d596c.svg)](https://classroom.github.com/a/YmMajyCa)

**PedidosRust** es un sistema distribuido implementado en Rust que modela la interacción entre *clientes*, *restaurantes*, *repartidores* y un *gateway de pagos*. Cada entidad funciona como una aplicación independiente, comunicándose mediante mensajes TCP.

La consigna del trabajo práctico puede encontrarse [aqui](https://concurrentes-fiuba.github.io/2025_1C_tp2.html) 

---

## Autores

| Nombre          | Apellido      | Mail                  | Padrón |
| --------------- | ------------- | --------------------- | ------ |
| Ian             | von der Heyde | ivon@fi.uba.ar        | 107638 |
| Agustín         | Altamirano    | aaltamirano@fi.uba.ar | 110237 |
| Juan Martín     | de la Cruz    | jdelacruz@fi.uba.ar   | 109588 |
| Santiago Tomás  | Fassio        | sfassio@fi.uba.ar     | 109463 |

---

## Índice

1. [Descripción general del sistema](#descripción-general-del-sistema)
   * [Características principales](#características-principales)
   * [Procesos del sistema](#procesos-del-sistema)
   * [Actores por cada proceso](#actores-por-proceso)
   * [Descripción de los mensajes](#descripción-de-los-mensajes)
6. [Instalación y Ejecución](#instalación-y-ejecución)
7. [Ejemplo de Ejecución](#ejemplo-de-ejecución)
8. [Pruebas](#pruebas)

---

## **Descripción general del sistema**

### **Características principales**

* **Modelo de Actores Asincrónicos**
  El sistema está construido siguiendo el **modelo de actores**, lo que permite una gestión eficiente y concurrente de mensajes entre múltiples entidades distribuidas. Cada componente del sistema (clientes, restaurantes, repartidores, servidores) está representado por actores independientes que se comunican de forma no bloqueante a través de TCP.

* **Coordinación distribuida y elección de coordinador**
  Se implementa el **algoritmo del anillo (Ring Algorithm)** para llevar a cabo la **elección de un administrador coordinador** entre los distintos procesos `Admin`. Este mecanismo garantiza que, ante la caída del coordinador actual, el sistema pueda elegir automáticamente un nuevo líder sin necesidad de intervención externa.

* **Exclusión Mutua Distribuida (Centralizada)**
  Para operaciones críticas que requieren acceso exclusivo a ciertos recursos (por ejemplo, actualización de datos globales), se utiliza un enfoque de **exclusión mutua distribuida centralizada**. El coordinador electo es el encargado de otorgar el permiso de acceso, garantizando consistencia y evitando condiciones de carrera entre los nodos.

* **Resiliencia y Tolerancia a Fallos**
  El sistema está diseñado con foco en la **tolerancia a fallos**, permitiendo que nodos individuales (como clientes, repartidores o restaurantes) puedan desconectarse temporalmente **sin afectar el flujo global del sistema**. Esta resiliencia se logra mediante:

  * **Heartbeats periódicos** entre procesos `Admin`, para detectar y responder rápidamente ante fallas.
  * **Backups sincronizados** del estado del sistema, asegurando persistencia y recuperación consistente.
  * **Soporte para reconexión de nodos**: los procesos pueden reconectarse automáticamente. Además, según el **estado actual de la orden**, es posible que ciertas operaciones (como la entrega de un pedido) continúen exitosamente **incluso si un cliente u otro nodo se encuentra momentáneamente desconectado**.

---

### **Procesos del Sistema**

El sistema está conformado por múltiples procesos independientes que se ejecutan en consolas separadas. Cada proceso representa un **nodo autónomo** dentro de la arquitectura distribuida del sistema, y se comunica mediante **mensajes TCP asincrónicos**.

#### Procesos principales

Los siguientes procesos representan las distintas funciones centrales del sistema:

* **PaymentGateway** — Puerto TCP: `8080`
* **Server1** — Puerto TCP: `8081`
* **Server2** — Puerto TCP: `8082`
* **Server3** — Puerto TCP: `8083`
* **Server4** — Puerto TCP: `8084`

Cada uno de estos servidores ejecuta un `Admin`, coordina actores internos y maneja conexiones con otros nodos del sistema.

#### Procesos dinámicos

Además, por cada entidad de negocio se lanza un proceso independiente:

* **Cliente** — Un proceso por cada cliente activo.
* **Restaurante** — Un proceso por cada restaurante disponible.
* **Delivery** — Un proceso por cada repartidor conectado.

Estos procesos se conectan dinámicamente a alguno de los `Server`, y se comunican de forma bidireccional para operar dentro del sistema (por ejemplo, iniciar pedidos, aceptar entregas, recibir actualizaciones, etc.).

---

### Actores por proceso

Cada proceso está compuesto por varios actores, cada uno con una responsabilidad específica. A continuación se describen los actores de cada proceso:

* [**Proceso Server**](#proceso-server): 
  * Acceptor
  * Admin
  * AdminCoordinator
  * OrderService
  * NearbyDeliveryService
  * NearbyRestaurantService
  * Storage

* [**Proceso PaymentGateway**](#proceso-paymentgateway):
   * Acceptor
   * TCP Sender
   * TCP Receiver
   * PaymentGateway

* [**Proceso Cliente**](#proceso-cliente):
   * TCP Sender
   * TCP Receiver
   * Client
   * UIHandler

* [**Proceso Restaurante**](#proceso-restaurante):
   * TCP Sender
   * TCP Receiver
   * Restaurant
   * OrderReceiver
   * Kitchen
   * Chef
   * DeliveryAssigner

* [**Proceso Delivery**](#proceso-delivery):
   * TCP Sender
   * TCP Receiver
   * Delivery

---

### Comunicación entre procesos: `TCP Sender` y `TCP Receiver`

La comunicación entre procesos distribuidos en este sistema se realiza a través de **mensajes TCP**. Para abstraer esta comunicación y mantener la lógica del sistema desacoplada del transporte subyacente, se utilizan dos actores especializados:

#### 📤 `TCPSender` *(Async)*

El `TCPSender` es el actor responsable de **enviar mensajes TCP** hacia otro nodo del sistema.

```rust
pub struct TCPSender {
    pub writer: Option<BufWriter<WriteHalf<TcpStream>>>,
}
```

Características:

* Utiliza un `BufWriter` sobre la mitad de escritura del socket (`WriteHalf<TcpStream>`).
* Recibe mensajes desde otros actores del sistema (por ejemplo, `Admin`, `Client`, etc.) y los escribe en el socket.
* Está diseñado para trabajar en paralelo con un `TCPReceiver` que lee de la misma conexión.

#### 📥 `TCPReceiver` *(Async)*

El `TCPReceiver` es el actor responsable de **leer mensajes entrantes desde un socket TCP** y reenviarlos al actor de destino adecuado dentro del sistema.

```rust
pub struct TCPReceiver {
    reader: Option<BufReader<ReadHalf<TcpStream>>>,
    destination: Addr<Actor>,
}
```

Características:

* Utiliza un `BufReader` sobre la mitad de lectura del socket (`ReadHalf<TcpStream>`).
* Deserializa cada línea recibida y la envía como mensaje al actor indicado mediante `destination`.
* Es genérico en cuanto al actor destino, lo que permite reutilizarlo en múltiples procesos (por ejemplo, `Client`, `Restaurant`, etc.).

#### 🔄 Emparejamiento mediante `Communicator`

Tanto el `TCP Sender` como el `TCP Receiver` están encapsulados dentro de una estructura llamada `Communicator`, que representa una **conexión lógica con otro nodo** (cliente, restaurante, delivery, otro servidor, o el Payment Gateway).

```rust
pub struct Communicator {
    pub sender: Addr<TCPSender>,
    pub receiver: Addr<TCPReceiver>,
    pub peer_type: PeerType, // Enum: Client, Restaurant, Delivery, Admin, Gateway
}
```

Este diseño permite que los distintos actores del sistema interactúen entre sí mediante mensajes, sin necesidad de preocuparse por la gestión directa de sockets o serialización.

---

### **Proceso `Server`**

Cada proceso `Server` representa un nodo del sistema. Cada uno de estos procesos se ejecuta en una consola diferente y se comunica a través de mensajes TCP.

A continuación, desarrollaremos en base al proceso `Server1` como ejemplo, pero el funcionamiento es el mismo para los otros procesos `Server`.


<p align="center">
  <img src="img/server_architecture.png" style="max-width: 100%; height: auto;" alt="Server Architecture">

</p>


---

#### 🔌 **Acceptor** *(Async)*

El actor **Acceptor** es responsable de escuchar el puerto TCP del proceso `Server`, aceptando conexiones entrantes desde diversos tipos de nodos del sistema: clientes, restaurantes, repartidores, otros servidores (`AdminX`) y el `Payment Gateway`.

Por cada nueva conexión aceptada, se instancian automáticamente los siguientes actores de comunicación:

* 📤 [`TCPSender`](#comunicación-entre-procesos-tcp-sender-y-tcp-receiver)
* 📥 [`TCPReceiver`](#comunicación-entre-procesos-tcp-sender-y-tcp-receiver)

Estos actores son los encargados de gestionar la entrada y salida de mensajes TCP entre el `Server` y el nodo conectado, desacoplando así la lógica de transporte del resto del sistema.

##### Estado interno del actor Acceptor

```rust
pub struct Acceptor {
    /// Puerto TCP donde escucha nuevas conexiones.
    pub listen_port: u16,
    /// Lista de conexiones activas.
    pub active_connections: HashSet<SocketAddr>,
}
```

---

#### 🧠 **Admin** *(Async)*

El actor **Admin** es el **componente central de coordinación** del proceso `Server`. Su función principal es recibir, interpretar y direccionar todos los mensajes entrantes del sistema.

Responsabilidades:

* Recibir mensajes provenientes de los `TCPReceiver`.
* Enviar mensajes hacia los `TCPSender` asociados a clientes, restaurantes, repartidores y al `Payment Gateway`.
* Coordinar acciones con los actores internos:

  * [`AdminCoordinator`](#🔗-admincoordinator-async)
  * [`OrderService`](#️⚙️-servicios-internos-async)
  * [`NearbyDeliveryService`](#️⚙️-servicios-internos-async)
  * [`NearbyRestaurantService`](#️⚙️-servicios-internos-async)
  * [`Storage`](#🗄️-storage-async)
  * [`Reaper`](#💀-reaper-async)

##### Estado interno del actor Admin

```rust
pub struct Admin {
  /// Coordinador actual.
  pub current_coordinator: Option<SocketAddr>,
  /// Estado de los pedidos en curso.
  pub active_orders: HashSet<u64>,
  /// Mapa de conexiones activas con clientes, restaurantes, deliverys y gateways.
  pub communicators: HashMap<SocketAddr, Communicator>,
  /// Canal de envío hacia el actor `Storage`. 
  pub storage: Addr<Storage>,
  /// Canal de envío hacia el actor `Reaper`.
  pub reaper: Addr<Reaper>,
  /// Servicio de órdenes.
  pub order_service: Addr<OrderService>,
  /// Servicio de restaurantes cercanos.
  pub nearby_restaurant_service: Addr<NearbyRestaurantService>,
  /// Servicio de deliverys cercanos.
  pub nearby_delivery_service: Addr<NearbyDeliveryService>,
}
```

---

#### 🔗 **AdminCoordinator** *(Async)*

El actor **AdminCoordinator** es el encargado de la **coordinación distribuida entre instancias del proceso `Server`** (Admins).

Este actor utiliza los `Communicator` previamente establecidos con `Admin2`, `Admin3` y `Admin4` para implementar:

* El algoritmo de **anillo (ring)** para la organización lógica de los servidores y elección de líder.
* Envío de **heartbeats** para detectar fallos.
* Sincronización periódica del estado del sistema (`Storage`) entre nodos.

##### Estado interno del actor AdminCoordinator

```rust
pub struct AdminCoordinator {
    /// Lista ordenada de nodos en el anillo.
    pub ring_nodes: Vec<SocketAddr>,
    /// Nodo coordinador actual.
    pub coordinator: Option<SocketAddr>,
    /// Timestamps de los últimos heartbeats recibidos por nodo.
    pub heartbeat_timestamps: HashMap<SocketAddr, Instant>,
}
```

---

#### ⚙️ **Servicios internos** *(Async)*

Los servicios internos se encargan de tareas especializadas dentro del proceso `Server`, accediendo al actor `Storage` para realizar lecturas y actualizaciones consistentes.

* **OrderService**
  Mantiene el estado de las órdenes en curso.
  Se comunica con: `Admin`, `Storage`.

* **NearbyRestaurantService**
  Identifica restaurantes cercanos a un cliente para iniciar el proceso de pedido.
  Se comunica con: `Admin`, `Storage`.

* **NearbyDeliveryService**
  Encuentra repartidores disponibles próximos a un restaurante para asignar la entrega.
  Se comunica con: `Admin`, `Storage`.

##### Estado interno de OrderService

```rust
pub struct OrderService {
   /// Mapa local de órdenes y sus estados.
   pub orders: HashMap<u64, OrderStatus>,
   /// Mapa local de clientes y su órden.
   pub clients_orders: HashMap<String, Vec<u64>>,
   /// Mapa local de restaurantes y sus órdenes.
   pub restaurants_orders: HashMap<String, Vec<u64>>,
   /// Cola de órdenes pendientes para procesamiento.
   pub pending_orders: Vec<u64>,
}
```

##### Estado interno de NearbyDeliveryService

```rust
pub struct NearbyDeliveryService {
   /// Cache local de repartidores disponibles con su ubicación.
   pub available_deliveries: HashMap<String, (f32, f32)>, // delivery_id -> posición (latitud, longitud)
}
```

##### Estado interno de NearbyRestaurantService

```rust
pub struct NearbyRestaurantService {
   /// Cache local de restaurantes disponibles con su ubicación.
   pub available_restaurants: HashMap<String, (f32, f32)>, // restaurant_id -> posición (latitud, longitud)
}
```

---

#### 🗄️ **Storage** *(Async)*

El actor **Storage** es responsable de la **persistencia del estado global** del sistema. Administra en memoria la información de entidades del sistema y permite acceder a ellas de forma segura y eficiente.

Gestiona:

* Información de clientes, restaurantes y repartidores.
* Estado detallado de cada orden.

Se comunica directamente con los siguientes actores:

* `Admin`
* `OrderService`
* `NearbyDeliveryService`
* `NearbyRestaurantService`

##### Estado interno del storage actor

```rust
pub struct ClientEntity {
    /// Posición actual del cliente en coordenadas 2D.
    pub client_position: (f32, f32),
    /// ID único del cliente.
    pub client_id: String,
    /// Pedido del cliente (id de alimento).
    pub client_order_id: Option<u64>,
    /// Marca de tiempo que registra la última actualización del cliente.
    pub time_stamp: Instant,
}

pub struct RestaurantEntity {
    /// Posición actual del restaurante en coordenadas 2D.
    pub restaurant_position: (f32, f32),
    /// ID único del restaurante.
    pub restaurant_id: String,
    /// Pedidos autorizados por el PaymentGatewat pero no aceptados todavía
    /// por el restaurante
    pub authorized_orders: HashSet<u64>,
    /// Pedidos pendientes.
    pub pending_orders: HashSet<u64>,
    /// Marca de tiempo que registra la última actualización del restaurante.
    pub time_stamp: Instant,
}

pub struct DeliveryEntity {
    /// Posición actual del delivery en coordenadas 2D.
    pub delivery_position: (f32, f32),
    /// ID único del delivery.
    pub delivery_id: String,
    /// ID del cliente actual asociado con el delivery (si existe).
    pub current_client_id: Option<String>,
    /// ID de la orden actual.
    pub current_order_id: Option<u64>,
    /// Estado actual del delivery.
    pub status: DeliveryStatus,
    /// Marca de tiempo que registra la última actualización del delivery.
    pub time_stamp: Instant,
}

pub struct OrderEntity {
    /// ID de la orden.
    pub order_id: u64,
    /// ID del cliente asociado a la orden.
    pub client_id: String,
    /// ID del restaurante asociado a la orden.
    pub restaurant_id: String,
    /// ID del delivery asociado a la orden.
    pub delivery_id: Option<String>,
    /// Estado de la orden.
    pub status: OrderStatus,
    /// Marca de tiempo que registra la última actualización de la orden.
    pub time_stamp: Instant,
}

pub struct Storage {
    /// Diccionario con información sobre clientes.
    pub clients: HashMap<SocketAddr, ClientEntity>,
    /// Diccionario con información sobre restaurantes.
    pub restaurants: HashMap<SocketAddr, RestaurantEntity>,
    /// Diccionario con información sobre deliverys.
    pub deliverys: HashMap<SocketAddr, DeliveryEntity>,
    /// Diccionario de órdenes.
    pub orders: HashMap<u64, OrderEntity>,
}
```

#### 💀 **Reaper** *(Async)*

```rust
/*
/
/
/
/
/
/
/  ¡ MISSING !
/
/
/
/
/
/
/
*/
```

---

### **Proceso `Cliente`**

Cada proceso `Cliente` representa a un comensal dentro del sistema. Se ejecuta en una consola independiente y se comunica únicamente con un proceso `Server` mediante mensajes TCP. Su función principal es realizar pedidos, esperar su procesamiento, y recibir notificaciones del estado de su orden.

El proceso está compuesto por dos actores principales:

* [`UIHandler`](#uihandler-async)
* [`Client`](#client-async)

### Tabla de estados del pedido (desde la perspectiva del Cliente)

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
| *Cualquiera intermedio* | Pedido cancelado en cualquier etapa | `CANCELLED`          | `Server → Client`    | Por rechazo de restaurante, problema con delivery u otra razón.     |


---

#### 🎛️ **UIHandler** *(Async)*

El actor **UIHandler** representa la interfaz de interacción humano-sistema. Su rol es recolectar inputs del usuario y mostrar por pantalla información relevante que llega desde el sistema.

Responsabilidades:

* Leer inputs del usuario (nombre, pedido y elección de restaurante).
* Mostrar mensajes y estados del pedido.
* Comunicarse con el actor `Client` enviando mensajes.

##### Estado interno de `UIHandler`

```rust
pub struct UIHandler {
  /// Canal de envío hacia el actor `Client`
  pub client: Addr<Client>,
}
```

---

#### 🧠 **Client** *(Async)*

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
  /// Identificador único del comensal
  pub client_id: String,
  /// Posición actual del cliente en coordenadas 2D
  pub position: (f32, f32),
  /// Estado actual del pedido (si hay uno en curso)
  pub order_status: Option<OrderStatus>,
  /// Restaurante elegido para el pedido
  pub selected_restaurant: Option<String>,
  /// ID del pedido actual
  pub order_id: Option<u64>,
  /// Canal de envío hacia el actor `UIHandler`
  pub ui_handler: Addr<UIHandler>,
  /// Comunicador asociado al `Server`
  pub communicator: Communicator,
}
```

---

#### **Proceso `Restaurante`** *(Async)*

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

### Tabla de estados del pedido (desde la perspectiva del Restaurante)

| Estado Inicial       | Acción del Restaurante                  | Estado Final             | Actor Responsable           | Comentario                                                |
| -------------------- | --------------------------------------- | ------------------------ | --------------------------- | --------------------------------------------------------- |
| `PENDING`            | Pedido recibido y encolado              | `PENDING`                | `OrderReceiver → Kitchen`   | Pasa directo a cocina.                                    |
| `AUTHORIZED`         | Restaurante lo rechaza                  | `CANCELLED`              | `OrderReceiver`             | Se envía `CancelOrder` al `Server`.                       |
| `AUTHORIZED`         | Restaurante lo acepta                   | `PENDING`                | `OrderReceiver → Kitchen`   | Se informa al `Server` (y este al `Client`) que fue aceptado. |
| `PENDING`            | Pedido asignado a chef                  | `PREPARING`              | `Kitchen → Server`          | Se informa al `Server` (y este al `Client`) que comenzó la preparación. |
| `PREPARING`          | Chef termina la cocción                 | `READY_FOR_DELIVERY`     | `Chef → DeliveryAssigner`   | Se informa al `Server` (y este al `Client`) que está listo para despachar.                 |
| `READY_FOR_DELIVERY` | Pedido asignado a un delivery           | `DELIVERING`             | `DeliveryAssigner → Server` | Se notifica al `Server` (y este al `Client`) con `DeliverThisOrder`.           |

---

#### **OrderReceiver** *(Async)*

Encargado de recibir nuevos pedidos provenientes del `Server` y reenviarlos al componente adecuado según su estado (`PENDING` o `AUTHORIZED`).

**Responsabilidades:**

* Conectarse al `Server` y realizar el proceso de `Recover`.
* Recibir nuevos pedidos desde el `Server`.
* Enviar directamente a `Kitchen` los pedidos `PENDING`.
* Para pedidos `AUTHORIZED`:

  * Confirmar (enviar a `Kitchen` + `UpdateOrderStatus(Pending)` al `Server`).
  * O rechazar (`CancelOrder` al `Server`).

##### Estado interno de `OrderReceiver`

```rust
pub struct OrderReceiver {
  /// Identificador único del restaurante
  pub restaurant_id: String,
  /// Posición actual del restaurante en coordenadas 2D
  pub position: (f32, f32),
  /// Canal de envío hacia el actor `Kitchen`
  pub kitchen_sender: Addr<Kitchen>,
  /// Comunicador asociado al `Server`
  pub communicator: Communicator,
}
```

---

#### **Kitchen** *(Async)*

Gestiona la cola de pedidos que deben prepararse y coordina a los chefs disponibles.

**Responsabilidades:**

* Mantener la cola de pedidos en espera.
* Asignar pedidos a chefs disponibles.
* Informar al `Server` cuando un pedido entra en estado `Preparing`.

##### Estado interno de `Kitchen`

```rust
pub struct Kitchen {
  /// Ordenes pendientes para ser preparadas.
  pub pending_orders: VecDeque<Order>,
  /// Chef disponible para preparar el pedido.
  pub chefs_available: Vec<Addr<Chef>>,
  /// Comunicador asociado al `Server`
  pub communicator: Communicator,
}
```

---

#### 🧑‍🍳 **Chef** *(Async)*

Simula la preparación de un pedido, demora un tiempo artificial y notifica cuando el pedido está listo para ser despachado.

**Responsabilidades:**

* Cocinar los pedidos asignados (delay simulado).
* Notificar al `DeliveryAssigner` con `SendThisOrder`.
* Avisar a la `Kitchen` que está disponible nuevamente (`IAmAvailable`).

##### Estado interno de `Chef`

```rust
pub struct Chef {
  /// Tiempo estimado para preparar pedidos
  pub time_to_cook: Duration,
  /// Pedido que está preparando
  pub order: Option<Order>,
  /// Canal de envío hacia el actor `Kitchen`
  pub kitchen_sender: Addr<Kitchen>,
  /// Canal de envío hacia el actor `DeliveryAssigner`
  pub delivery_assigner: Addr<DeliveryAssigner>
}
```

---

#### 🚴 **DeliveryAssigner** *(Async)*

Encargado de pedir repartidores al `Server` y asociarlos con pedidos listos para entregar.

**Responsabilidades:**

* Encolar pedidos listos para despacho.
* Solicitar deliverys al `Server`.
* Manejar llegadas de `DeliveryAvailable`.
* Enviar `DeliverThisOrder` al `Server`.

##### Estado interno de `DeliveryAssigner`

```rust
pub struct DeliveryAssigner {
  /// Queue de pedidos listos para ser despachados.
  pub ready_orders: VecDeque<Order>,
  /// Mapa de ordenes enviadas y su delivery asignado.
  pub orders_delivery: HashMap<u64, String>,
  /// Comunicador asociado al `Server`
  pub communicator: Communicator,
}
```

---

#### **Proceso `Delivery`** *(Async)*

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

### Tabla de estados del Delivery

| Estado Actual          | Evento o Acción                     | Nuevo Estado           | Acción del Delivery                        | Comentario                                                                 |
| ---------------------- | ----------------------------------- | ---------------------- | ------------------------------------------ | -------------------------------------------------------------------------- |
| `INITIAL`              | Se lanza el proceso                 | `RECONNECTING`         | Establece conexión con `Server`            | Comienza el descubrimiento de coordinador (`who is coord?`).               |
| `RECONNECTING`         | Se conecta al `Server`              | `RECOVERING`           | Enviar `Recover(delivery_id)`              | Informa su `delivery_id` y solicita estado previo.                         |
| `RECOVERING`           | Respuesta con datos de entrega      | `DELIVERING`           | Reanuda entrega pendiente                  | Retoma un pedido que había quedado en curso.                               |
| `RECOVERING`           | Respuesta sin datos                 | `AVAILABLE`            | Enviar `IAmAvailable(delivery_id, pos)`    | No estaba entregando, se registra como disponible.                         |
| `AVAILABLE`            | Recibe `NewOfferToDeliver`          | `OCCUPIED (tentative)` | Si acepta: enviar `AcceptedOrder(order)`   | Si no acepta, ignora el mensaje y sigue disponible.                        |
| `OCCUPIED (tentative)` | Recibe `DeliveryNoNeeded`           | `AVAILABLE`            | Espera o decide reconectarse más adelante  | Otro delivery fue asignado más rápido.                                     |
| `OCCUPIED (tentative)` | Recibe `DeliverThisOrder`           | `DELIVERING`           | Inicia simulación de entrega               | Confirmación final de asignación del pedido.                               |
| `DELIVERING`           | Termina la entrega (viaje simulado) | `AVAILABLE`            | Enviar `Delivered(order)` + `IAmAvailable` | Informa finalización y vuelve a estar disponible para nuevas asignaciones. |

---

#### **Delivery** *(Async)*

El actor `Delivery` encapsula toda la lógica de un repartidor. Mantiene su estado interno (ubicación, ocupación actual, pedido activo si lo hubiera) y se comunica exclusivamente con el `Server`.

**Responsabilidades:**

* Realizar el proceso de `Recover` para detectar si tiene un pedido en curso.
* Reportar disponibilidad al `Server`.
* Evaluar ofertas de entrega y responder si está libre.
* Ejecutar la entrega una vez confirmada por el `Server`.
* Simular el tiempo de viaje y finalizar el pedido.

##### Estado interno de `Delivery`

```rust
pub struct Delivery {
  /// Identificador único del delivery
  pub delivery_id: String,
  /// Posición actual del delivery
  pub position: (f32, f32),
  /// Estado actual del delivery: Disponible, Ocupado, Entregando
  pub status: DeliveryStatus,
  /// Pedido actual en curso, si lo hay
  pub current_order: Option<Order>,
  /// Comunicador asociado al Server
  pub communicator: Communicator,
}
```

##### Enum `DeliveryStatus`

```rust
pub enum DeliveryStatus {
  Available,    // Listo para recibir ofertas de pedidos
  Occupied,     // Esperando confirmación final
  Delivering,   // En proceso de entrega
}
```
