use actix::dev::ToEnvelope;
use actix::prelude::*;
use tokio::io::{AsyncBufReadExt, BufReader, ReadHalf};
use tokio::net::TcpStream;

#[derive(Message)]
#[rtype(result = "()")]
pub struct IncomingLine(pub String);

pub struct SocketReader<A: Actor + Handler<IncomingLine>> {
    reader: Option<BufReader<ReadHalf<TcpStream>>>,
    destination: Addr<A>,
}

impl<A> SocketReader<A>
where
    A: Actor + Handler<IncomingLine>,
{
    pub fn new(reader: ReadHalf<TcpStream>, destination: Addr<A>) -> Self {
        Self {
            reader: Some(BufReader::new(reader)),
            destination,
        }
    }
}

impl<A> Actor for SocketReader<A>
where
    A: Actor + Handler<IncomingLine> + 'static,
    A::Context: ToEnvelope<A, IncomingLine>,
{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = self.destination.clone();
        let reader = self.reader.take().unwrap();

        ctx.spawn(
            async move {
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    addr.do_send(IncomingLine(line));
                }
            }
            .into_actor(self),
        );
    }
}