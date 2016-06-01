use std::io;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use futures::{self, Future};

use std::sync::Arc;

use {Handler, IntoHandler, Request, Response};

pub struct Mux {
    handler_map: HashMap<PathBuf, Handler>,
}

impl Mux {
    pub fn new() -> Mux {
        Mux { handler_map: HashMap::new() }
    }

    // Always replaces the handler.
    pub fn register<F: IntoHandler, P: Into<PathBuf>> (&mut self, path: P, handler: F) {
        self.handler_map.insert(path.into(), handler.into_handler());
    }

    fn get_handler<P: AsRef<Path>>(&self, path: P) -> Option<&Handler> {
        let path = path.as_ref();
        let mut components = path.components();

        loop {
            if let res@Some(_) = self.handler_map.get(components.as_path()) {
                return res;
            }
            if components.next_back().is_none() {
                break;
            }
        }

        None
    }
}

fn not_found(req: Request) -> Box<Future<Item=Response, Error=io::Error>> {
        let mut resp = Response::new();
        let body = format!("NOPE NOT HERE: {}", req.path());
        resp.header("Content-Type", "text/plain")
            .header("Content-Lenth", &body.len().to_string())
            .header("Server", "wut")
            .body(&body);
        futures::finished(resp).boxed()
}

impl IntoHandler for Mux {
    fn into_handler(self) -> Handler {
        Arc::new(move |r| {
            if let Some(h) = self.get_handler(r.path()) {
                h(r)
            } else {
                not_found(r)
            }
        })
    }
}
