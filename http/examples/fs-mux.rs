#[macro_use] extern crate log;

extern crate env_logger;
extern crate http;
extern crate time;
extern crate futures;

use std::path::Path;
use std::io;

use futures::*;
use http::{Mux, Request, Response};

fn main() {
    env_logger::init().unwrap();

    let mut mux = Mux::new();
    mux.register("/hello", |_r| {
        let mut r = Response::new();
        r.header("Content-Type", "text/plain")
         .header("Content-Lenth", "15")
         .header("Server", "wut")
         .header("Date", &time::now().rfc822().to_string())
         .body("Hello, World!");
        finished(r).boxed()
    });
    mux.register("/hello/rust", |_r| {
        let mut r = Response::new();
        r.header("Content-Type", "text/plain")
         .header("Content-Lenth", "14")
         .header("Server", "wut")
         .header("Date", &time::now().rfc822().to_string())
         .body("Hello, Rust!");
        finished(r).boxed()
    });
    mux.register("/headers", |req: Request| {
        let mut resp = Response::new();
        let header_strs: Vec<_> =
            req.headers()
               .map(|(h, v)| { format!("{}: {}", h, String::from_utf8_lossy(v)) })
               .collect();
        let body = header_strs.join("\r\n");
        resp.header("Content-Type", "text/plain")
            .header("Content-Length", &body.len().to_string())
            .header("Server", "wut")
            .header("Date", &time::now().rfc822().to_string())
            .body(body.as_str());
        finished(resp).boxed()
    });
    mux.register("/files", |req: Request| {
        let relative = Path::new(req.path()).strip_prefix("/files").unwrap();
        let path = Path::new("/").join(relative);
        debug!("Request for file {:?}", path);
        fs::File::open(path).and_then(|f| {
            let buf = Vec::with_capacity(1024);
            f.read(buf)
        }).and_then(|buf| {
            let mut resp = Response::new();
            let body = String::from_utf8_lossy(buf.as_slice());
            resp.header("Content-Type", "text/plain")
                .header("Content-Lenth", &body.len().to_string())
                .header("Server", "wut")
                .header("Date", &time::now().rfc822().to_string())
                .body(&body);
            finished(resp).boxed()
        }).map_err(|e| io::Error::from(e)).boxed()
    });
    http::serve(&"127.0.0.1:8080".parse().unwrap(), mux);
}
