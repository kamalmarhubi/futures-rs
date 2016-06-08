use std::fs;
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;
use std::slice;
use std::sync::Arc;

use executor::{Executor, ThreadPool};
use {Future, promise};

lazy_static! {
    // TODO: pick a better number; what does libuv do?
    static ref POOL: ThreadPool = ThreadPool::new(10);
}

// TODO: unduplicate from futuremio.
pub struct Error<T> {
    err: io::Error,
    data: T,
}

impl<T> Error<T> {
    pub fn new(err: io::Error, data: T) -> Error<T> {
        Error {
            err: err,
            data: data,
        }
    }

    pub fn into_pair(self) -> (io::Error, T) {
        (self.err, self.data)
    }
}

impl<T> From<Error<T>> for io::Error {
    fn from(e: Error<T>) -> io::Error {
        e.err
    }
}

impl<T: Default> From<io::Error> for Error<T> {
    fn from(e: io::Error) -> Error<T> {
        Error::new(e, Default::default()) 
    }
}

pub struct File {
    inner: Arc<fs::File>
}

impl From<fs::File> for File {
    fn from(f: fs::File) -> File {
        File { inner: Arc::new(f) }
    }
}

impl File {
    // TODO: error type shouldn't mention Vec.
    pub fn open(path: PathBuf) -> Box<Future<Item=File, Error=Error<Vec<u8>>>>
    {
        let (p, c) = promise();
        POOL.execute(move || {
            match fs::File::open(&path) {
                Ok(f) => c.finish(File::from(f)),
                Err(e) => c.fail(e.into()),
            }
        });
        p.boxed()
    }

    pub fn close(self) -> Box<Future<Item=(), Error=io::Error>> {
        unimplemented!();
    }

    pub fn read(&self, mut into: Vec<u8>) -> Box<Future<Item=Vec<u8>, Error=Error<Vec<u8>>>> {
        // TODO: unduplicate from futuremio.
        unsafe fn slice_to_end(v: &mut Vec<u8>) -> &mut [u8] {
            slice::from_raw_parts_mut(v.as_mut_ptr().offset(v.len() as isize),
            v.capacity() - v.len())
        }
        let (p, c) = promise();
        let f = self.inner.clone();
        POOL.execute(move || {
            debug!("File::read: in pool for {:?}", f);
            let r = unsafe {
                (&*f).read(slice_to_end(&mut into))
            };
            debug!("File::read: done reading for {:?}", f);
            match r {
                Ok(i) => {
                    unsafe {
                        let len = into.len();
                        into.set_len(len + i);
                    }
                    debug!("File::read: calling c.finish for {:?}", f);
                    c.finish(into);
                }
                Err(e) => {
                    c.fail(Error::new(e, into));
                }
            }
        });

        p.boxed()
    }
}
