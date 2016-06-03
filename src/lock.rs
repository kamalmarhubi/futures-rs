use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::Ordering::{Acquire, Release};
use std::sync::atomic::AtomicBool;

/// A lock for protecting shared data.
///
/// This lock will not block threads attempting to acquire it. To take the lock, call
/// [`try_lock`](#method.try_lock), which will either succeed or fail.
pub struct Lock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

/// An RAII guard for holding a lock. When dropped, the lock is released.
///
/// The data protected by the lock can be accessed through this guard via `Deref` and `DerefMut`
/// implementations.
pub struct TryLock<'a, T: 'a> {
    __ptr: &'a Lock<T>,
}

unsafe impl<T: Send> Send for Lock<T> {}
unsafe impl<T: Send> Sync for Lock<T> {}

impl<T> Lock<T> {
    /// Creates a new lock in an unlocked state.
    pub fn new(t: T) -> Lock<T> {
        Lock {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(t),
        }
    }

    /// Attempts to acquire the lock.
    ///
    /// If the lock could not be acquired, returns `None`, otherwise an RAII guard is returned. The
    /// lock will be released when the guard is dropped.
    pub fn try_lock(&self) -> Option<TryLock<T>> {
        if !self.locked.swap(true, Acquire) {
            Some(TryLock { __ptr: self })
        } else {
            None
        }
    }
}

impl<'a, T> Deref for TryLock<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.__ptr.data.get() }
    }
}

impl<'a, T> DerefMut for TryLock<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.__ptr.data.get() }
    }
}

impl<'a, T> Drop for TryLock<'a, T> {
    fn drop(&mut self) {
        self.__ptr.locked.store(false, Release);
    }
}

#[cfg(test)]
mod tests {
    use super::Lock;

    #[test]
    fn smoke() {
        let a = Lock::new(1);
        let mut a1 = a.try_lock().unwrap();
        assert!(a.try_lock().is_none());
        assert_eq!(*a1, 1);
        *a1 = 2;
        drop(a1);
        assert_eq!(*a.try_lock().unwrap(), 2);
        assert_eq!(*a.try_lock().unwrap(), 2);
    }
}
