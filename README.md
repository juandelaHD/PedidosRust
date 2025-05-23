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
   * [Características Principales](#características-principales)
   * [Procesos (consolas)](#procesos-del-sistema)
   * [Actores por cada proceso](#actores-por-proceso)
   * [Descripción de los mensajes](#descripción-de-los-mensajes)
6. [Instalación y Ejecución](#instalación-y-ejecución)
7. [Ejemplo de Ejecución](#ejemplo-de-ejecución)
8. [Pruebas](#pruebas)

---

Claro, acá tenés una versión más clara, formal y completa de la **Descripción general del sistema**, incorporando lo que mencionás sobre exclusión mutua distribuida y el algoritmo del anillo:

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

* **Server1** — Puerto TCP: `8080`
* **Server2** — Puerto TCP: `8081`
* **Server3** — Puerto TCP: `8082`
* **Server4** — Puerto TCP: `8083`
* **PaymentGateway** — Puerto TCP: `8084`

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
Claro, aquí tenés una versión más clara y profesionalmente redactada del segmento correspondiente al proceso **Server** y sus actores:

---

#### 🔌 **Acceptor** *(Async)*

El actor **Acceptor** es responsable de escuchar el puerto TCP del proceso `Server`, aceptando conexiones entrantes desde diversos tipos de nodos del sistema: clientes, restaurantes, repartidores, otros servidores (`AdminX`) y el `Payment Gateway`.

Por cada nueva conexión aceptada, se instancian automáticamente los siguientes actores de comunicación:

* 📤 [`TCPSender`](#comunicación-entre-procesos-tcp-sender-y-tcp-receiver)
* 📥 [`TCPReceiver`](#comunicación-entre-procesos-tcp-sender-y-tcp-receiver)

Estos actores son los encargados de gestionar la entrada y salida de mensajes TCP entre el `Server` y el nodo conectado, desacoplando así la lógica de transporte del resto del sistema.

---

#### 🧠 **Admin** *(Async)*

El actor **Admin** es el **componente central de coordinación** del proceso `Server`. Su función principal es recibir, interpretar y direccionar todos los mensajes entrantes del sistema.

Responsabilidades:

* Recibir mensajes provenientes de los `TCPReceiver`.
* Enviar mensajes hacia los `TCPSender` asociados a clientes, restaurantes, repartidores y al `Payment Gateway`.
* Coordinar acciones con los actores internos:

  * [`OrderService`](#️servicios-internos-async)
  * [`NearbyDeliveryService`](#️servicios-internos-async)
  * [`NearbyRestaurantService`](#️servicios-internos-async)
  * [`AdminCoordinator`](#admincoordinator-async)
  * [`Storage`](#storage-async)

---

#### 🔗 **AdminCoordinator** *(Async)*

El actor **AdminCoordinator** es el encargado de la **coordinación distribuida entre instancias del proceso `Server`** (Admins).

Este actor utiliza los `Communicator` previamente establecidos con `Admin2`, `Admin3` y `Admin4` para implementar:

* El algoritmo de **anillo (ring)** para la organización lógica de los servidores.
* Envío de **heartbeats** para detectar fallos.
* Sincronización periódica del estado del sistema (`Storage`) entre nodos.

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
      /// ID unico del cliente
      pub client_id: String,
      /// pedido del cliente (id de alimento)
      pub client_order_id: Option<u64>,
      /// Marca de tiempo que registra la última actualización del cliente.
      pub time_stamp: Instant,
   }

   pub struct RestaurantEntity {
      /// Posición actual del restaurante en coordenadas 2D.
      pub restaurant_position: (f32, f32),
      /// ID unico del restaurante
      pub restaurant_id: String,
      /// Pedidos pendientes.
      pub pending_orders: HashSet<u64>,
      /// Pedidos listos.
      pub ready_orders: HashSet<u64>,
      /// Marca de tiempo que registra la última actualización del restaurant.
      pub time_stamp: Instant,
   }

   pub struct DeliveryEntity {
      /// Posición actual del delivery en coordenadas 2D.
      pub delivery_position: (f32, f32),
      /// ID único del delivery
      pub delivery_id: String,
      /// ID del cliente actual asociado con el delivery (si existe).
      pub current_client_id: Option<String>,
      /// ID de la orden actual
      pub current_order_id: Option<u64>
      /// Estado actual del delivery.
      pub status: DeliveryStatus,
      /// Marca de tiempo que registra la última actualización del delivery.
      pub time_stamp: Instant,
   }

   pub struct OrderEntity {
      /// ID de la orden
      pub order_id: u64,
      /// ID del cliente asociado a la orden
      pub client_id: String,
      /// ID del restaurante asociado a la orden
      pub restaurant_id: String,
      /// ID del delivery asociado a la orden
      pub delivery_id: Option<String>,
      /// Estado de la orden
      pub status: OrderStatus,
      /// Marca de tiempo que registra la última actualización de la orden.
      pub time_stamp: Instant,
   }

   pub struct Storage {
      /// Diccionario con informacion sobre clientes
      pub clients: HashMap<SocketAddr, ClientEntity>,
      /// Diccionario con informacion sobre restaurantes
      pub restaurants: HashMap<SocketAddr, RestaurantEntity>,
      /// Diccionario con informacion sobre deliverys
      pub deliverys: HashMap<SocketAddr, DeliveryEntity>,
      /// Diccionario de ordenes
      pub orders: HashMap<u64, OrderEntity>,
   }
```

