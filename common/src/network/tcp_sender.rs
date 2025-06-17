// use crate::messages::shared_messages::NetworkMessage;
// use actix::prelude::*;
// use std::collections::VecDeque;
// use tokio::io::{AsyncWriteExt, BufWriter, WriteHalf};
// use tokio::net::TcpStream;

// pub struct TCPSender {
//     pub writer: Option<BufWriter<WriteHalf<TcpStream>>>,
//     pub queue: VecDeque<NetworkMessage>,
// }

// impl TCPSender {
//     pub fn new(write_half: WriteHalf<TcpStream>) -> Self {
//         Self {
//             writer: Some(BufWriter::new(write_half)),
//             queue: VecDeque::new(),
//         }
//     }
// }

// impl Actor for TCPSender {
//     type Context = Context<Self>;
// }

// struct ProcessQueue;

// impl Message for ProcessQueue {
//     type Result = ();
// }

// impl Handler<NetworkMessage> for TCPSender {
//     type Result = ();

//     fn handle(&mut self, msg: NetworkMessage, ctx: &mut Self::Context) {
//         self.queue.push_back(msg);
//         if self.queue.len() == 1 {
//             ctx.notify(ProcessQueue);
//         }
//     }
// }

// impl Handler<ProcessQueue> for TCPSender {
//     type Result = ResponseActFuture<Self, ()>;

//     fn handle(&mut self, _msg: ProcessQueue, _ctx: &mut Self::Context) -> Self::Result {
//         if let (Some(mut writer), Some(msg)) = (self.writer.take(), self.queue.front().cloned()) {
//             let fut = async move {
//                 let serialized = match serde_json::to_string(&msg) {
//                     Ok(s) => s,
//                     Err(e) => panic!("Error serializing message: {:?}", e),
//                 };
//                 let to_send = format!("{}\n", serialized);
//                 if let Err(e) = writer.write_all(to_send.as_bytes()).await {
//                     panic!("Error writing to socket: {:?}", e);
//                 }
//                 if let Err(e) = writer.flush().await {
//                     panic!("Error flushing socket: {:?}", e);
//                 }
//                 writer
//             };
//             Box::pin(fut.into_actor(self).map(|writer, act, ctx| {
//                 act.writer = Some(writer);
//                 act.queue.pop_front();
//                 if !act.queue.is_empty() {
//                     ctx.notify(ProcessQueue);
//                 }
//             }))
//         } else {
//             Box::pin(async {}.into_actor(self))
//         }
//     }
// }

use crate::messages::shared_messages::NetworkMessage;
use actix::prelude::*;
use std::collections::VecDeque;
use tokio::io::{AsyncWriteExt, BufWriter, WriteHalf};
use tokio::net::TcpStream;

pub struct TCPSender {
    pub writer: Option<BufWriter<WriteHalf<TcpStream>>>,
    pub queue: VecDeque<NetworkMessage>,
}

impl TCPSender {
    pub fn new(write_half: WriteHalf<TcpStream>) -> Self {
        Self {
            writer: Some(BufWriter::new(write_half)),
            queue: VecDeque::new(),
        }
    }
}

/// Mensaje para indicar error en el socket, que se puede propagar al supervisor.
#[derive(Message)]
#[rtype(result = "()")]
pub struct SendError(pub String);

impl Actor for TCPSender {
    type Context = Context<Self>;
}

struct ProcessQueue;

impl Message for ProcessQueue {
    type Result = ();
}

impl Handler<NetworkMessage> for TCPSender {
    type Result = ();

    fn handle(&mut self, msg: NetworkMessage, ctx: &mut Self::Context) {
        self.queue.push_back(msg);
        println!(
            "[TCPSender] Message added to queue, queue size: {}",
            self.queue.len()
        );
        println!("[TCPSender] Current queue: {:?}", self.queue);
        if self.queue.len() == 1 {
            ctx.notify(ProcessQueue);
        }
    }
}

impl Handler<ProcessQueue> for TCPSender {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: ProcessQueue, ctx: &mut Self::Context) -> Self::Result {
        if let (Some(mut writer), Some(msg)) = (self.writer.take(), self.queue.front().cloned()) {
            let addr = ctx.address();

            let fut = async move {
                let serialized = match serde_json::to_string(&msg) {
                    Ok(s) => s,
                    Err(e) => {
                        // No panic, se puede loguear o manejar el error.
                        let err = format!("Error serializing message: {:?}", e);
                        return Err(err);
                    }
                };
                let to_send = format!("{}\n", serialized);

                if let Err(e) = writer.write_all(to_send.as_bytes()).await {
                    let err = format!("Error writing to socket: {:?}", e);
                    return Err(err);
                }
                if let Err(e) = writer.flush().await {
                    let err = format!("Error flushing socket: {:?}", e);
                    return Err(err);
                }

                Ok(writer)
            };

            Box::pin(fut.into_actor(self).map(move |res, act, ctx| {
                match res {
                    Ok(writer) => {
                        act.writer = Some(writer);
                        act.queue.pop_front();
                        if !act.queue.is_empty() {
                            ctx.notify(ProcessQueue);
                        }
                    }
                    Err(err_msg) => {
                        act.writer = None; // Forzamos cierre para evitar usar writer inválido
                        act.queue.clear(); // Opcional: limpiar cola porque hay error

                        // Loguear error (o enviar a otro actor supervisor)
                        println!("[TCPSender] {}", err_msg);

                        // Por ejemplo, enviar mensaje a supervisor o hacer self-stop:
                        // ctx.stop();

                        // O emitir mensaje para que quien controle la conexión maneje reconexión
                        // addr.do_send(SendError(err_msg));
                    }
                }
            }))
        } else {
            Box::pin(async {}.into_actor(self))
        }
    }
}
