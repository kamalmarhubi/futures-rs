use std::cell::{Cell, RefCell};
use std::sync::Arc;
use std::thread;

use crossbeam::sync::MsQueue;

pub trait Executor: Send + Sync + 'static {
    fn execute<F>(&self, f: F)
        where F: FnOnce() + Send + 'static,
              Self: Sized
    {
        self.execute_boxed(Box::new(f))
    }

    fn execute_boxed(&self, f: Box<ExecuteCallback>);
}

pub static DEFAULT: Limited = Limited;

impl<T: Executor + ?Sized + Send + Sync + 'static> Executor for Box<T> {
    fn execute_boxed(&self, f: Box<ExecuteCallback>) {
        (**self).execute_boxed(f)
    }
}

impl<T: Executor + ?Sized + Send + Sync + 'static> Executor for Arc<T> {
    fn execute_boxed(&self, f: Box<ExecuteCallback>) {
        (**self).execute_boxed(f)
    }
}

pub trait ExecuteCallback: Send + 'static {
    fn call(self: Box<Self>);
}

impl<F: FnOnce() + Send + 'static> ExecuteCallback for F {
    fn call(self: Box<F>) {
        (*self)()
    }
}

pub struct Inline;

impl Executor for Inline {
    fn execute<F: FnOnce() + Send + 'static>(&self, f: F) {
        f()
    }

    fn execute_boxed(&self, f: Box<ExecuteCallback>) {
        f.call()
    }
}

pub struct Limited;

thread_local!(static LIMITED: LimitState = LimitState::new());

const LIMIT: usize = 100;

struct LimitState {
    count: Cell<usize>,
    deferred: RefCell<Vec<Box<ExecuteCallback>>>,
}

impl Executor for Limited {
    fn execute<F>(&self, f: F) where F: FnOnce() + Send + 'static {
        LIMITED.with(|state| state.execute(f))
    }
    fn execute_boxed(&self, f: Box<ExecuteCallback>) {
        self.execute(|| f.call());
    }
}

impl LimitState {
    fn new() -> LimitState {
        LimitState {
            count: Cell::new(0),
            deferred: RefCell::new(Vec::new()),
        }
    }

    fn execute<F>(&self, f: F) where F: FnOnce() + Send + 'static {
        match self.count.get() {
            0 => {
                self.count.set(1);
                f();
                loop {
                    let cb = self.deferred.borrow_mut().pop();
                    match cb {
                        Some(f) => f.call(),
                        None => break,
                    }
                }
                self.count.set(0);
            }
            n if n < LIMIT => {
                self.count.set(n + 1);
                f();
                self.count.set(n);
            }
            _ => self.deferred.borrow_mut().push(Box::new(f)),
        }
    }
}

// TODO: handle panicked workers.
// TODO: shutdown workers on drop.
pub struct ThreadPool {
    queue: Arc<MsQueue<Box<ExecuteCallback>>>,
}

impl ThreadPool {
    pub fn new(num_threads: usize) -> ThreadPool {
        let queue = Arc::new(MsQueue::<Box<ExecuteCallback>>::new());
        for _ in 0..num_threads {
            let q = queue.clone();
            thread::spawn(move || {
                loop {
                    let work = q.pop();
                    work.call();
                }
            });
        }

        ThreadPool { queue: queue }
    }
}

impl Executor for ThreadPool {
    fn execute<F: FnOnce() + Send + 'static>(&self, f: F) {
        self.execute_boxed(Box::new(f))
    }

    fn execute_boxed(&self, f: Box<ExecuteCallback>) {
        self.queue.push(f)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Condvar, Mutex};
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{Executor, Limited, ThreadPool};

    #[test]
    fn limited() {
        fn doit(ex: Arc<Executor>, hits: Arc<AtomicUsize>, i: usize) {
            if i == 0 {
                return
            }
            hits.fetch_add(1, Ordering::SeqCst);
            let ex2 = ex.clone();
            ex.execute(move || {
                doit(ex2, hits, i - 1);
            })
        }

        let n = 1_000_000;
        let hits = Arc::new(AtomicUsize::new(0));
        doit(Arc::new(Limited), hits.clone(), n);
        assert_eq!(hits.load(Ordering::SeqCst), n);
    }

    #[test]
    fn threadpool() {
        const N: usize = 1_000;
        fn doit(ex: Arc<Executor>, pair: Arc<(Mutex<usize>, Condvar)>, i: usize) {
            if i == 0 {
                return
            }
            {
                let &(ref lock, ref cvar) = &*pair;
                let mut hits = lock.lock().unwrap();
                *hits += 1;
                if *hits >= N {
                    cvar.notify_all()
                }
            }

            let ex2 = ex.clone();
            ex.execute(move || {
                doit(ex2, pair, i - 1);
            })
        }

        let pair = Arc::new((Mutex::new(0), Condvar::new()));
        let pool = ThreadPool::new(10);
        doit(Arc::new(pool), pair.clone(), N);

        let &(ref lock, ref cvar) = &*pair;
        let mut hits = lock.lock().unwrap();
        while *hits < N {
            hits = cvar.wait(hits).unwrap();
        }

        assert_eq!(*hits, N);
    }
}
