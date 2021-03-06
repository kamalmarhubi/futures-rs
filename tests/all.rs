extern crate futures;

use std::sync::mpsc::{channel, TryRecvError};
use std::fmt;

use futures::*;

fn unwrap<A, B>(r: PollResult<A, B>) -> Result<A, B> {
    match r {
        Ok(e) => Ok(e),
        Err(PollError::Canceled) => panic!("future canceled"),
        Err(PollError::Panicked(p)) => panic!(p),
        Err(PollError::Other(e)) => Err(e),
    }
}


fn f_ok(a: i32) -> Done<i32, u32> { Ok(a).into_future() }
fn f_err(a: u32) -> Done<i32, u32> { Err(a).into_future() }
fn ok(a: i32) -> Result<i32, u32> { Ok(a) }
fn err(a: u32) -> Result<i32, u32> { Err(a) }

fn assert_done<T, F>(mut f: F, result: Result<T::Item, T::Error>)
    where T: Future,
          T::Item: Eq + fmt::Debug,
          T::Error: Eq + fmt::Debug,
          F: FnMut() -> T,
{
    // let mut a = f();
    // assert_eq!(&unwrap(a.poll().expect("future not ready")), &result);
    // assert_bad(a.poll().expect("future should still be ready"));

    let (tx, rx) = channel();
    f().schedule(move |r| tx.send(r).unwrap());
    assert_eq!(&unwrap(rx.recv().unwrap()), &result);

    let mut a = f();
    a.schedule(|_| ());
    let (tx, rx) = channel();
    a.schedule(move |r| tx.send(r).unwrap());
    assert_panic(rx.recv().unwrap());
}

fn assert_empty<T: Future, F: FnMut() -> T>(mut f: F) {
    // let mut a = f();
    // assert!(a.poll().is_none());
    // a.cancel();
    // assert_cancel(a.poll().expect("cancel should force a finish"));

    // let mut a = f();
    // a.cancel();
    // assert_cancel(a.poll().expect("cancel should force a finish2"));

    let (tx, rx) = channel();
    let mut a = f();
    a.schedule(move |r| drop(tx.send(r)));
    assert!(rx.try_recv().is_err());
    drop(a);

    let (tx, rx) = channel();
    let (tx2, rx2) = channel();
    let mut a = f();
    a.schedule(move |r| tx.send(r).unwrap());
    a.schedule(move |r| tx2.send(r).unwrap());
    assert_panic(rx2.recv().unwrap());
    drop(a);
    assert_cancel(rx.recv().unwrap());

    let (tx, rx) = channel();
    let mut a = f();
    let tx2 = tx.clone();
    a.schedule(move |r| tx2.send(r).unwrap());
    a.schedule(move |r| tx.send(r).unwrap());
    assert_bad(rx.recv().unwrap());
    drop(a);
    assert_cancel(rx.recv().unwrap());
}

fn assert_cancel<T, E>(r: PollResult<T, E>) {
    match r {
        Ok(_) => panic!("can't succeed after cancel"),
        Err(PollError::Canceled) => {}
        Err(PollError::Panicked(_)) => panic!("panic error, not cancel"),
        Err(PollError::Other(_)) => panic!("other error, not cancel"),
    }
}

fn assert_panic<T, E>(r: PollResult<T, E>) {
    match r {
        Ok(_) => panic!("can't succeed after panic"),
        Err(PollError::Canceled) => panic!("cancel error, not panic"),
        Err(PollError::Panicked(_)) => {}
        Err(PollError::Other(_)) => panic!("other error, not panic"),
    }
}

fn assert_bad<T, E>(r: PollResult<T, E>) {
    match r {
        Ok(_) => panic!("expected panic, got success"),
        Err(PollError::Other(_)) => panic!("expected panic, got other"),
        Err(PollError::Panicked(_)) => {}
        Err(PollError::Canceled) => {}
    }
}

fn unselect<T, U, E>(r: Result<(T, U), (E, U)>) -> Result<T, E> {
    match r {
        Ok((t, _)) => Ok(t),
        Err((e, _)) => Err(e),
    }
}

#[test]
fn result_smoke() {
    fn is_future_v<A, B, C>(_: C)
        where A: Send + 'static,
              B: Send + 'static,
              C: Future<Item=A, Error=B>
    {}

    is_future_v::<i32, u32, _>(f_ok(1).map(|a| a + 1));
    is_future_v::<i32, u32, _>(f_ok(1).map_err(|a| a + 1));
    is_future_v::<i32, u32, _>(f_ok(1).and_then(|a| Ok(a)));
    is_future_v::<i32, u32, _>(f_ok(1).or_else(|a| Err(a)));
    is_future_v::<(i32, i32), u32, _>(f_ok(1).join(Err(3)));
    is_future_v::<i32, u32, _>(f_ok(1).map(move |a| f_ok(a)).flatten());

    assert_done(|| f_ok(1), ok(1));
    assert_done(|| f_err(1), err(1));
    assert_done(|| done(Ok(1)), ok(1));
    assert_done(|| done(Err(1)), err(1));
    assert_done(|| finished(1), ok(1));
    assert_done(|| failed(1), err(1));
    assert_done(|| f_ok(1).map(|a| a + 2), ok(3));
    assert_done(|| f_err(1).map(|a| a + 2), err(1));
    assert_done(|| f_ok(1).map_err(|a| a + 2), ok(1));
    assert_done(|| f_err(1).map_err(|a| a + 2), err(3));
    assert_done(|| f_ok(1).and_then(|a| Ok(a + 2)), ok(3));
    assert_done(|| f_err(1).and_then(|a| Ok(a + 2)), err(1));
    assert_done(|| f_ok(1).and_then(|a| Err(a as u32 + 3)), err(4));
    assert_done(|| f_err(1).and_then(|a| Err(a as u32 + 4)), err(1));
    assert_done(|| f_ok(1).or_else(|a| Ok(a as i32 + 2)), ok(1));
    assert_done(|| f_err(1).or_else(|a| Ok(a as i32 + 2)), ok(3));
    assert_done(|| f_ok(1).or_else(|a| Err(a + 3)), ok(1));
    assert_done(|| f_err(1).or_else(|a| Err(a + 4)), err(5));
    assert_done(|| f_ok(1).select(f_err(2)).then(unselect), ok(1));
    assert_done(|| f_ok(1).select(Ok(2)).then(unselect), ok(1));
    assert_done(|| f_err(1).select(f_ok(1)).then(unselect), err(1));
    assert_done(|| f_ok(1).select(empty()).then(unselect), Ok(1));
    assert_done(|| empty().select(f_ok(1)).then(unselect), Ok(1));
    assert_done(|| f_ok(1).join(f_err(1)), Err(1));
    assert_done(|| f_ok(1).join(Ok(2)), Ok((1, 2)));
    assert_done(|| f_err(1).join(f_ok(1)), Err(1));
    assert_done(|| f_ok(1).then(|_| Ok(2)), ok(2));
    assert_done(|| f_ok(1).then(|_| Err(2)), err(2));
    assert_done(|| f_err(1).then(|_| Ok(2)), ok(2));
    assert_done(|| f_err(1).then(|_| Err(2)), err(2));
}

#[test]
fn test_empty() {
    fn empty() -> Empty<i32, u32> { futures::empty() }

    assert_empty(|| empty());
    assert_empty(|| empty().select(empty()));
    assert_empty(|| empty().join(empty()));
    assert_empty(|| empty().join(f_ok(1)));
    assert_empty(|| f_ok(1).join(empty()));
    assert_empty(|| empty().or_else(move |_| empty()));
    assert_empty(|| empty().and_then(move |_| empty()));
    assert_empty(|| f_err(1).or_else(move |_| empty()));
    assert_empty(|| f_ok(1).and_then(move |_| empty()));
    assert_empty(|| empty().map(|a| a + 1));
    assert_empty(|| empty().map_err(|a| a + 1));
    assert_empty(|| empty().then(|a| a));
}

#[test]
fn test_finished() {
    assert_done(|| finished(1), ok(1));
    assert_done(|| failed(1), err(1));
}

#[test]
fn flatten() {
    fn finished<T: Send + 'static>(a: T) -> Finished<T, u32> {
        futures::finished(a)
    }
    fn failed<E: Send + 'static>(b: E) -> Failed<i32, E> {
        futures::failed(b)
    }

    assert_done(|| finished(finished(1)).flatten(), ok(1));
    assert_done(|| finished(failed(1)).flatten(), err(1));
    assert_done(|| failed(1u32).map(finished).flatten(), err(1));
    assert_done(|| futures::finished::<_, u8>(futures::finished::<_, u32>(1))
                           .flatten(), ok(1));
    assert_empty(|| finished(empty::<i32, u32>()).flatten());
    assert_empty(|| empty::<i32, u32>().map(finished).flatten());
}

#[test]
fn smoke_promise() {
    assert_done(|| {
        let (p, c) = promise();
        c.finish(1);
        p
    }, ok(1));
    assert_done(|| {
        let (p, c) = promise();
        c.fail(1);
        p
    }, err(1));
    let mut completes = Vec::new();
    assert_empty(|| {
        let (a, b) = promise::<i32, u32>();
        completes.push(b);
        a
    });

    let (mut p, c) = promise::<i32, u32>();
    drop(c);
    p.schedule(assert_cancel);
    p.schedule(assert_panic);
    let (tx, rx) = channel();
    p.schedule(move |r| tx.send(r).unwrap());
    assert_panic(rx.recv().unwrap());
}

#[test]
fn select_cancels() {
    let ((a, b), (c, d)) = (promise::<i32, u32>(), promise::<i32, u32>());
    let ((atx, arx), (ctx, crx)) = (channel(), channel());
    let a = a.map(move |a| { atx.send(a).unwrap(); a });
    let c = c.map(move |c| { ctx.send(c).unwrap(); c });

    let mut f = a.select(c).then(unselect);
    // assert!(f.poll().is_none());
    assert!(arx.try_recv().is_err());
    assert!(crx.try_recv().is_err());
    b.finish(1);
    f.schedule(|_| ());
    // assert!(f.poll().is_some());
    assert_eq!(arx.recv().unwrap(), 1);
    drop((d, f));
    assert!(crx.recv().is_err());

    let ((a, b), (c, d)) = (promise::<i32, u32>(), promise::<i32, u32>());
    let ((atx, _arx), (ctx, crx)) = (channel(), channel());
    let a = a.map(move |a| { atx.send(a).unwrap(); a });
    let c = c.map(move |c| { ctx.send(c).unwrap(); c });

    let mut f = a.select(c).then(unselect);
    f.schedule(|_| ());
    f.schedule(assert_panic);
    // assert_panic(f.poll().unwrap());
    b.finish(1);
    drop((d, f));
    assert!(crx.recv().is_err());
}

#[test]
fn join_cancels() {
    let ((a, b), (c, d)) = (promise::<i32, u32>(), promise::<i32, u32>());
    let ((atx, _arx), (ctx, crx)) = (channel(), channel());
    let a = a.map(move |a| { atx.send(a).unwrap(); a });
    let c = c.map(move |c| { ctx.send(c).unwrap(); c });

    let mut f = a.join(c);
    b.fail(1);
    f.schedule(|_| ());
    // assert!(f.poll().is_some());
    drop((d, f));
    assert!(crx.recv().is_err());

    let ((a, b), (c, d)) = (promise::<i32, u32>(), promise::<i32, u32>());
    let ((atx, _arx), (ctx, crx)) = (channel(), channel());
    let a = a.map(move |a| { atx.send(a).unwrap(); a });
    let c = c.map(move |c| { ctx.send(c).unwrap(); c });

    let mut f = a.join(c);
    f.schedule(|_| ());
    f.schedule(assert_panic);
    b.fail(1);
    drop((d, f));
    assert!(crx.recv().is_err());
}

#[test]
fn join_incomplete() {
    let (a, b) = promise::<i32, u32>();
    let mut f = f_ok(1).join(a);
    // assert!(f.poll().is_none());
    let (tx, rx) = channel();
    f.schedule(move |r| tx.send(r).unwrap());
    assert!(rx.try_recv().is_err());
    b.finish(2);
    assert_eq!(unwrap(rx.recv().unwrap()), Ok((1, 2)));

    let (a, b) = promise::<i32, u32>();
    let mut f = a.join(f_ok(2));
    // assert!(f.poll().is_none());
    let (tx, rx) = channel();
    f.schedule(move |r| tx.send(r).unwrap());
    assert!(rx.try_recv().is_err());
    b.finish(1);
    assert_eq!(unwrap(rx.recv().unwrap()), Ok((1, 2)));

    let (a, b) = promise::<i32, u32>();
    let mut f = f_ok(1).join(a);
    // assert!(f.poll().is_none());
    let (tx, rx) = channel();
    f.schedule(move |r| tx.send(r).unwrap());
    assert!(rx.try_recv().is_err());
    b.fail(2);
    assert_eq!(unwrap(rx.recv().unwrap()), Err(2));

    let (a, b) = promise::<i32, u32>();
    let mut f = a.join(f_ok(2));
    // assert!(f.poll().is_none());
    let (tx, rx) = channel();
    f.schedule(move |r| tx.send(r).unwrap());
    assert!(rx.try_recv().is_err());
    b.fail(1);
    assert_eq!(unwrap(rx.recv().unwrap()), Err(1));
}

#[test]
fn cancel_propagates() {
    let mut f = promise::<i32, u32>().0.then(|_| -> Done<i32, u32> { panic!() });
    f.schedule(|r| assert_cancel(r));
    let mut f = promise::<i32, u32>().0.and_then(|_| -> Done<i32, u32> { panic!() });
    f.schedule(|r| assert_cancel(r));
    let mut f = promise::<i32, u32>().0.or_else(|_| -> Done<i32, u32> { panic!() });
    f.schedule(|r| assert_cancel(r));
    let mut f = promise::<i32, u32>().0.map(|_| panic!());
    f.schedule(|r| assert_cancel(r));
    let mut f = promise::<i32, u32>().0.map_err(|_| panic!());
    f.schedule(|r| assert_cancel(r));
}

#[test]
fn collect_collects() {
    assert_done(|| collect(vec![f_ok(1), f_ok(2)]), Ok(vec![1, 2]));
    assert_done(|| collect(vec![f_ok(1)]), Ok(vec![1]));
    assert_done(|| collect(Vec::<Result<i32, u32>>::new()), Ok(vec![]));

    // TODO: needs more tests
}

#[test]
fn select2() {
    fn d<T, U, E>(r: Result<(T, U), (E, U)>) -> Result<T, E> {
        match r {
            Ok((t, _u)) => Ok(t),
            Err((e, _u)) => Err(e),
        }
    }

    assert_done(|| f_ok(2).select(empty()).then(d), Ok(2));
    assert_done(|| empty().select(f_ok(2)).then(d), Ok(2));
    assert_done(|| f_err(2).select(empty()).then(d), Err(2));
    assert_done(|| empty().select(f_err(2)).then(d), Err(2));

    assert_done(|| {
        f_ok(1).select(f_ok(2))
               .map_err(|_| 0)
               .and_then(|(a, b)| b.map(move |b| a + b))
    }, Ok(3));

    // Finish one half of a select and then fail the second, ensuring that we
    // get the notification of the second one.
    {
        let ((a, b), (c, d)) = (promise::<i32, u32>(), promise::<i32, u32>());
        let mut f = a.select(c);
        let (tx, rx) = channel();
        f.schedule(move |r| tx.send(r).unwrap());
        b.finish(1);
        let (val, mut next) = rx.recv().unwrap().ok().unwrap();
        assert_eq!(val, 1);
        let (tx, rx) = channel();
        next.schedule(move |r| tx.send(r).unwrap());
        assert_eq!(rx.try_recv().err().unwrap(), TryRecvError::Empty);
        d.fail(2);
        match rx.recv().unwrap() {
            Err(PollError::Other(2)) => {}
            _ => panic!("wrong error"),
        }
    }

    // Fail the second half and ensure that we see the first one finish
    {
        let ((a, b), (c, d)) = (promise::<i32, u32>(), promise::<i32, u32>());
        let mut f = a.select(c);
        let (tx, rx) = channel();
        f.schedule(move |r| tx.send(r).unwrap());
        d.fail(1);
        let mut next = match rx.recv().unwrap() {
            Err(PollError::Other((1, next))) => next,
            _ => panic!("wrong result"),
        };
        let (tx, rx) = channel();
        next.schedule(move |r| tx.send(r).unwrap());
        assert_eq!(rx.try_recv().err().unwrap(), TryRecvError::Empty);
        b.finish(2);
        assert_eq!(rx.recv().unwrap().ok().unwrap(), 2);
    }

    // Cancelling the first half should cancel the second
    {
        let ((a, _b), (c, _d)) = (promise::<i32, u32>(), promise::<i32, u32>());
        let ((atx, arx), (ctx, crx)) = (channel(), channel());
        let a = a.map(move |v| { atx.send(v).unwrap(); v });
        let c = c.map(move |v| { ctx.send(v).unwrap(); v });
        let f = a.select(c);
        drop(f);
        assert!(crx.recv().is_err());
        assert!(arx.recv().is_err());
    }

    // Cancel after a schedule
    {
        let ((a, _b), (c, _d)) = (promise::<i32, u32>(), promise::<i32, u32>());
        let ((atx, arx), (ctx, crx)) = (channel(), channel());
        let a = a.map(move |v| { atx.send(v).unwrap(); v });
        let c = c.map(move |v| { ctx.send(v).unwrap(); v });
        let mut f = a.select(c);
        f.schedule(|_| ());
        drop(f);
        assert!(crx.recv().is_err());
        assert!(arx.recv().is_err());
    }

    // Cancel propagates
    {
        let ((a, b), (c, _d)) = (promise::<i32, u32>(), promise::<i32, u32>());
        let ((atx, arx), (ctx, crx)) = (channel(), channel());
        let a = a.map(move |v| { atx.send(v).unwrap(); v });
        let c = c.map(move |v| { ctx.send(v).unwrap(); v });
        let (tx, rx) = channel();
        let mut f = a.select(c).map(move |_| tx.send(()).unwrap());
        f.schedule(|_| ());
        drop(b);
        assert!(crx.recv().is_err());
        assert!(arx.recv().is_err());
        assert!(rx.recv().is_err());
    }

    // Cancel on early drop
    {
        let (tx, rx) = channel();
        let mut f = f_ok(1).select(empty().map(move |()| {
            tx.send(()).unwrap();
            1
        }));
        f.schedule(|_| ());
        drop(f);
        assert!(rx.recv().is_err());
    }
}
