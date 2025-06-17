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
                    Err(e) => panic!("Error serializing message: {:?}", e),
                };
                let to_send = format!("{}\n", serialized);
                if let Err(e) = writer.write_all(to_send.as_bytes()).await {
                    panic!("Error writing to socket: {:?}", e);
                }
                if let Err(e) = writer.flush().await {
                    panic!("Error flushing socket: {:?}", e);
                }
                writer
            };
            Box::pin(fut.into_actor(self).map(|writer, act, ctx| {
                act.writer = Some(writer);
                act.queue.pop_front();
                if !act.queue.is_empty() {
                    ctx.notify(ProcessQueue);
                }
            }))
        } else {
            Box::pin(async {}.into_actor(self))
        }
    }
}
