use std::sync::Arc;

use Callback;
use slot::Slot;
use stream::{Stream, StreamResult};
use util;

pub struct Map<S, F> {
    stream: S,
    f: Arc<Slot<F>>,
}

pub fn new<S, F>(s: S, f: F) -> Map<S, F> where F: Send + 'static {
    Map {
        stream: s,
        f: Arc::new(Slot::new(Some(f))),
    }
}

impl<S, F, U> Stream for Map<S, F>
    where S: Stream,
          F: FnMut(S::Item) -> U + Send + 'static,
          U: Send + 'static,
{
    type Item = U;
    type Error = S::Error;

    fn schedule<G>(&mut self, g: G)
        where G: FnOnce(StreamResult<Self::Item, Self::Error>) + Send + 'static,
    {
        let mut f = match util::opt2poll(self.f.try_consume().ok()) {
            Ok(f) => f,
            Err(e) => return g(Err(e)),
        };
        let slot = self.f.clone();
        self.stream.schedule(move |res| {
            let (f, res) = match res {
                Ok(Some(e)) => {
                    match util::recover(|| (f(e), f)) {
                        Ok((r, f)) => (Some(f), Ok(Some(r))),
                        Err(e) => (None, Err(e)),
                    }
                }
                Ok(None) => (Some(f), Ok(None)),
                Err(e) => (Some(f), Err(e)),
            };
            if let Some(f) = f {
                slot.try_produce(f).ok().expect("map stream failed to produce");
            }
            g(res)
        })
    }

    fn schedule_boxed(&mut self,
                      g: Box<Callback<Option<Self::Item>, Self::Error>>) {
        self.schedule(|r| g.call(r))
    }
}

