#[macro_use] extern crate log;
#[macro_use] extern crate lazy_static;
extern crate env_logger;
extern crate http;
extern crate time;
extern crate futures;

use std::path::PathBuf;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::*;
use http::Response;

lazy_static! {
    static ref COUNT: AtomicUsize = AtomicUsize::new(0);
}

fn main() {
    env_logger::init().unwrap();
    http::serve(&"127.0.0.1:8080".parse().unwrap(), |r| {
        debug!("GET {:?}", r.path());
        COUNT.fetch_add(1, Ordering::Relaxed);
        fs::File::open(PathBuf::from(r.path())).and_then(|f| {
            let buf = Vec::with_capacity(1024);
            f.read(buf)
        }).and_then(|b| {
            let mut r = Response::new();
            let body = String::from_utf8_lossy(b.as_slice());
            r.header("Content-Type", "text/plain")
             .header("Content-Lenth", &body.len().to_string())
             .header("Server", "wut")
             .header("X-Request-Count", &COUNT.load(Ordering::Relaxed).to_string())
             .header("Date", &time::now().rfc822().to_string())
             .body(&body);
            finished(r).boxed()
        }).map_err(|e| io::Error::from(e)).boxed()
    });
}
