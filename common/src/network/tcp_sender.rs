use crate::messages::shared_messages::NetworkMessage;
use crate::messages::shared_messages::Shutdown;
use actix::prelude::*;
use std::collections::VecDeque;
use tokio::io::{AsyncWriteExt, BufWriter, WriteHalf};
use tokio::net::TcpStream;

/// The `TCPSender` actor is responsible for serializing and sending [`NetworkMessage`]s
/// over a TCP stream to a remote peer. It maintains a queue to ensure messages are sent in order.
pub struct TCPSender {
    /// The buffered writer for the TCP stream.
    pub writer: Option<BufWriter<WriteHalf<TcpStream>>>,
    /// The queue of messages to be sent.
    pub queue: VecDeque<NetworkMessage>,
}

impl TCPSender {
    /// Creates a new `TCPSender` with the given write half of a TCP stream.
    pub fn new(write_half: WriteHalf<TcpStream>) -> Self {
        Self {
            writer: Some(BufWriter::new(write_half)),
            queue: VecDeque::new(),
        }
    }
}

/// Message sent to indicate an error occurred while sending data over the socket.
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
        if self.queue.len() == 1 {
            ctx.notify(ProcessQueue);
        }
    }
}

impl Handler<ProcessQueue> for TCPSender {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: ProcessQueue, _ctx: &mut Self::Context) -> Self::Result {
        if let (Some(mut writer), Some(msg)) = (self.writer.take(), self.queue.front().cloned()) {
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
                        eprintln!("[TCPSender] {}", err_msg);
                    }
                }
            }))
        } else {
            Box::pin(async {}.into_actor(self))
        }
    }
}

impl Handler<Shutdown> for TCPSender {
    type Result = ();

    fn handle(&mut self, _msg: Shutdown, ctx: &mut Self::Context) {
        self.writer = None;
        self.queue.clear();
        ctx.stop();
    }
}
