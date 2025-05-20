use actix::prelude::*;
use actix_async_handler::async_handler;
use tokio::io::{AsyncWriteExt, BufWriter, WriteHalf};
use tokio::net::TcpStream;

#[derive(Message)]
#[rtype(result = "()")]
pub struct TCPMessage(pub String);

pub struct SocketWriter {
    pub writer: Option<BufWriter<WriteHalf<TcpStream>>>,
}

impl SocketWriter {
    pub fn new(write_half: WriteHalf<TcpStream>) -> Self {
        Self {
            writer: Some(BufWriter::new(write_half)),
        }
    }
}

impl Actor for SocketWriter {
    type Context = Context<Self>;
}

#[async_handler]
impl Handler<TCPMessage> for SocketWriter {
    type Result = ();

    async fn handle(&mut self, msg: TCPMessage, _ctx: &mut Self::Context) -> Self::Result {
        let to_send = format!("{}\n", msg.0);

        if let Some(mut writer) = self.writer.take() {
            let ret_writer = async move {
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
            }
            .await;

            self.writer = ret_writer;
        }
    }
}
