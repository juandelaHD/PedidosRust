# Trabajo Práctico 2 - Concurrentes - FIUBA

## Introducción

PedidosRust es una nueva aplicación para conectar restaurantes, repartidores y comensales. 
Gracias a su innovadora implementación distribuida, permitirá reducir los costos y apuntar a ser líder en el mercado.

Los comensales podrán solicitar un pedido a un restaurante, los restaurantes notifican cuando el pedido está listo, y los repartidores buscan pedidos cercanos y los entregan.

### Objetivo

Deberán implementar un conjunto de aplicaciones en Rust que modele el sistema.

### Requerimientos

- Una aplicación modelará la app para los comensales. El comensal elegirá dentro de los restaurantes cercanos donde desea realizar el pedido. Por simplificación la ubicación se modelará con un par de enteros pequeños. Por ejemplo el comensal puede encontrarse ubicado en (6,8) y elegir un restaurante ubicado en (4,7). El pedido se compondrá simplemente por el monto a cobrar por el mismo. La app mantendrá notificado al comensal sobre la evolución de su pedido: cuando el restaurante lo empieza a preparar, cuando está listo, cuando lo recoge el repartidor y luego cuando el repartidor llegó y entregó el pedido. Cada comensal realizando pedidos en el sistema tendrá una instancia de la app corriendo independiente.

- Otra aplicación será la de los restaurantes. El restaurante tendrá una cola de pedidos que va recibiendo, de los cuales deberá ir informando su progreso. Cuando empieza a cocinarlos, cuando se encuentran listos para ser retirados. Así mismo el restaurante puede cancelar un pedido, si por ejemplo no tuviera suficiente stock (la decisión se modelará de forma aleatoria o por input, no es necesario un control verdadero de stocks). El restaurante preparará varios pedidos concurrentemente. Cada restaurante tendrá una instancia de la aplicación.

- Una aplicación más será para los repartidores. Los mismos informan su ubicación actual y reciben ofertas de pedidos listos para entregar cercanos. Varios repartidores pueden recibir la misma oferta concurrentemente. El primero en aceptarla se la queda. La aceptación o rechazo se modelará con un random o input para pruebas. Una vez aceptado tomará un tiempo hasta llegar al restaurante, donde tomará el paquete y lo informará en la app, y luego otro hasta llegar al domicilio del comensal y realiza la entrega; iniciando el ciclo nuevamente.

- Se debe considerar la autorización del pago al momento de realizar el pedido por parte del comensal, y el cobro efectivo al momento de llegar este a su domicilio. Modelar el gateway de pagos como una aplicación simple que loguea. Considerar que al momento de autorizar el pago, el gateway puede rechazar la tarjeta aleatoriamente con una determinada probabilidad.

- El sistema debe ser resiliente, soportando la caída de algunas instancias de las aplicaciones.

- El sistema debe intentar minimizar la cantidad de mensajes que viajan por toda la red; por ejemplo enviando mensajes sólo a los nodos que se encuentren cercanos por ubicación.

## Requerimientos no funcionales

- El proyecto deberá ser desarrollado en lenguaje **Rust**, usando las herramientas de la biblioteca estándar.
- Por lo menos alguna, si no todas las aplicaciones implementadas deben funcionar utilizando el **modelo de actores**.
- Cada instancia de cada aplicación debe ejecutarse en un proceso independiente.
- En el modelado de la solución se deberán utilizar **al menos dos** de las herramientas de concurrencia distribuida mostradas en la cátedra. Por ejemplo: exclusiones mutuas distribuidas, elección de líder, algoritmos de anillo, commits de dos fases, etc.
- No se permite utilizar **crates** externos, salvo los explícitamente mencionados en este enunciado, los utilizados en las clases, o los autorizados expresamente por los profesores.
- El código fuente debe compilarse en la última versión stable del compilador y no se permite utilizar bloques `unsafe`.
- El código deberá funcionar en ambiente Unix / Linux.
- El programa deberá ejecutarse en la línea de comandos.
- La compilación no debe arrojar **warnings** del compilador, ni del linter **clippy**.
- Las funciones y los tipos de datos (**`struct`**) deben estar documentadas siguiendo el estándar de **`cargo doc`**.
- El código debe formatearse utilizando **`cargo fmt`**.
- Cada tipo de dato implementado debe ser colocado en una unidad de compilación (archivo fuente) independiente.

## Entregas

La resolución del presente proyecto es en grupos de cuatro integrantes. No se aceptan grupos de otra cantidad de miembros.

Las entregas del proyecto se realizarán mediante Github Classroom. Cada grupo tendrá un repositorio disponible para hacer diferentes commits con el objetivo de resolver el problema propuesto.

### Primera entrega: Diseño

Deberán entregar un informe en formato Markdown en el `README.md` del repositorio que contenga una explicación del diseño y de las decisiones tomadas para la implementación de la solución, así como diagramas de threads y procesos, y la comunicación entre los mismos; y diagramas de las entidades principales.

De cada entidad se debe describir:
- Finalidad general
- Su estado interno, como uno o varios structs en pseudocódigo de Rust.
- Mensajes que recibe, struct del payload de los mismos y cómo reaccionará cuando los recibe.
- Mensajes que envía, struct del payload de los mismos y hacia quienes son enviados.
- Protocolos de transporte (TCP/UDP) y de aplicación que utiliza para comunicarse.
- Casos de interés (felices, caídas)

Se recomienda fuertemente que las implementaciones de las ideas de diseño se encuentren realizadas mínimamente, para validar el mismo.

**Fecha máxima de entrega: 28 de Mayo de 2025**

Presentar como un pull‑request del README.md.

El diseño será evaluado por la cátedra y de no presentarse a término el trabajo quedará automáticamente desaprobado.

La cátedra podrá solicitar correcciones que deberán realizarse en el mismo diseño, y puntos de mejora que deberán tenerse en cuenta durante la implementación.

Se podrán hacer commits hasta el día de la entrega a las 19 hs Arg, luego el sistema automáticamente quitará el acceso de escritura.

### Segunda entrega: Código final - 17 de Junio de 2025

- Deberá incluirse el código de la solución completa.
- Deberá actualizarse el readme, con una sección dedicada a cambios que se hayan realizado desde la primera entrega. Además debe incluir cualquier explicación y/o set de comandos necesarios para la ejecución de los programas.

### Presentación final: 17, 18, 24 y 25 de Junio

Cada grupo presentará presencialmente a un profesor de la cátedra su trabajo. La fecha y horario para cada grupo serán definidos oportunamente.

La presentación deberá incluir un resumen del diseño de la solución y la muestra en vivo de las aplicaciones funcionando, demostrando casos de interés.

Todos los integrantes del grupo deberán exponer algo. La evaluación es individual.

La cátedra podrá solicitar luego correcciones donde se encuentren errores severos, sobre todo en el uso de herramientas de concurrencia; o bien desviaciones respecto al diseño pactado inicialmente.

De tener correcciones para realizar, las mismas deben realizarse y aprobarse con anterioridad a la presentación a examen final.

## Evaluación

### Principios teóricos y corrección de bugs

Los estudiantes presentarán el diseño, código y comportamiento en vivo de su solución, con foco en el uso de las diferentes herramientas de concurrencia.

Deberán poder explicar, desde los conceptos teóricos vistos en clase, cómo se comportará potencialmente su solución ante problemas de concurrencia (por ejemplo ausencia de deadlocks).

En caso de que la solución no se comportara de forma esperada, deberán poder explicar las causas y sus posibles rectificaciones.

### Casos de prueba

Se someterá a la aplicación a diferentes casos de prueba que validen la correcta aplicación de las herramientas de concurrencia y la resiliencia de las distintas entidades.

### Informe

El README de su repositorio funcionará como informe del trabajo y deberá contener toda la información requerida tanto para la primera como para la segunda entrega.

### Organización del código

El código debe organizarse respetando los criterios de buen diseño y en particular aprovechando las herramientas recomendadas por Rust. Se prohíbe el uso de bloques `unsafe`.

### Tests automatizados

La entrega debe contar con tests automatizados que prueben diferentes casos. Se considerará en especial aquellos que pongan a prueba el uso de las herramientas de concurrencia.

### Presentación en término

El trabajo deberá entregarse para la fecha estipulada. La presentación fuera de término sin coordinación con antelación con el profesor influirá negativamente en la nota final.
