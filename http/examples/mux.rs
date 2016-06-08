extern crate http;
extern crate time;
extern crate futures;

use futures::*;
use http::{Mux, Request, Response};

fn main() {
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
    http::serve(&"127.0.0.1:8080".parse().unwrap(), mux);
}
