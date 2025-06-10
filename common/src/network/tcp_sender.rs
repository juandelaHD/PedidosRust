use crate::messages::shared_messages::NetworkMessage;
use actix::prelude::*;
use tokio::io::{AsyncWriteExt, BufWriter, WriteHalf};
use tokio::net::TcpStream;

pub struct TCPSender {
    pub writer: Option<BufWriter<WriteHalf<TcpStream>>>,
}

impl TCPSender {
    pub fn new(write_half: WriteHalf<TcpStream>) -> Self {
        Self {
            writer: Some(BufWriter::new(write_half)),
        }
    }
}

impl Actor for TCPSender {
    type Context = Context<Self>;
}

impl Handler<NetworkMessage> for TCPSender {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: NetworkMessage, _ctx: &mut Self::Context) -> Self::Result {
        // Serializar el mensaje usando serde_json
        let serialized = match serde_json::to_string(&msg) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error serializando NetworkMessage: {:?}", e);
                return Box::pin(async {}.into_actor(self));
            }
        };
        let to_send = format!("{}\n", serialized);

        if let Some(mut writer) = self.writer.take() {
            let fut = async move {
                if let Err(e) = writer.write_all(to_send.as_bytes()).await {
                    if e.kind() == std::io::ErrorKind::BrokenPipe {
                        // Pipe roto: limpiar writer
                        return None;
                    } else {
                        panic!("Error inesperado al enviar datos: {:?}", e);
                    }
                }
                // Importante hacer flush para asegurar que se envíen los datos
                if let Err(e) = writer.flush().await {
                    if e.kind() == std::io::ErrorKind::BrokenPipe {
                        return None;
                    } else {
                        panic!("Error inesperado al hacer flush: {:?}", e);
                    }
                }
                Some(writer)
            };
            Box::pin(fut.into_actor(self).map(|ret_writer, act, _ctx| {
                act.writer = ret_writer;
            }))
        } else {
            Box::pin(async {}.into_actor(self))
        }
    }
}
