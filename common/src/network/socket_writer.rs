use actix::prelude::*;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::net::tcp::OwnedWriteHalf;

#[derive(Message)]
#[rtype(result = "()")]
pub struct TCPMessage(pub String);

pub struct SocketWriter {
    writer: Option<BufWriter<OwnedWriteHalf>>,
}

impl SocketWriter {
    pub fn new(writer: OwnedWriteHalf) -> Self {
        Self {
            writer: Some(BufWriter::new(writer)),
        }
    }
}

impl Actor for SocketWriter {
    type Context = Context<Self>;
}

impl Handler<TCPMessage> for SocketWriter {
    type Result = ();

    fn handle(&mut self, msg: TCPMessage, _ctx: &mut Self::Context) {
        if let Some(writer) = self.writer.as_mut() {
            let data = msg.0 + "\n";
            let _ = writer.write_all(data.as_bytes());
            let _ = writer.flush();
        }
    }
}