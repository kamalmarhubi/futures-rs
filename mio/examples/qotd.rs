extern crate futuremio;
extern crate futures;

use std::env;
use std::net::SocketAddr;

use futures::Future;
use futuremio::{IoFuture, Loop, TcpListener, TcpStream};

// RFC 865 :-)
const QOTD_PORT: u16 = 17;

fn main() {
    let port = env::args().nth(1).map(|v| v.parse().unwrap()).unwrap_or(QOTD_PORT);

    let mut l = Loop::new().unwrap();
    let listener = l.tcp_listen(
        &SocketAddr::new(
            "127.0.0.1".parse().unwrap(),
            port)).unwrap();

    l.await(accept(listener)).unwrap();
}

fn accept(listener: TcpListener) -> Box<IoFuture<()>> {
    listener.accept().and_then(move |(stream, _addr)| {
        send_quote(stream)
            .join(accept(listener))
            .map(|_| ())
            .boxed()
    }).boxed()
}

fn send_quote(stream: TcpStream) -> Box<IoFuture<()>> {
    let quote = b"If I knew any quotes, I'd've put one here.".to_vec();
    stream.write(0, quote).map(|_| ()).map_err(From::from).boxed()
}
