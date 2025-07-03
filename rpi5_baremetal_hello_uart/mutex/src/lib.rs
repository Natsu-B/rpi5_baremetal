#![cfg_attr(not(test), no_std)]

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    // TODO critical section (no interrupt?)
    pub fn lock(&'_ self) -> SpinLockGuard<'_, T> {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        SpinLockGuard { lock: self }
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

pub struct RWLock<T> {
    /// 最上位ビットをwrite flagとして利用する
    /// その他のビットはread countとして利用
    ///
    /// readではwrite flagとwrite待ちflagがなければread countを増やして読む
    /// writeではすべてのbitが立っていなければ書き込み、write flagを立てて待つ
    read_count_write_lock_flag: AtomicUsize,
    data: UnsafeCell<T>,
}

pub struct RWLockReadGuard<'a, T> {
    lock: &'a RWLock<T>,
}

pub struct RWLockWriteGuard<'a, T> {
    lock: &'a RWLock<T>,
}

unsafe impl<T: Send + Sync> Sync for RWLock<T> {}
unsafe impl<T: Send> Send for RWLock<T> {}

impl<T> RWLock<T> {
    const WRITE_FLAG: usize = 1 << (usize::BITS - 1);
    pub const fn new(data: T) -> Self {
        Self {
            read_count_write_lock_flag: AtomicUsize::new(0),
            data: UnsafeCell::new(data),
        }
    }

    pub fn read(&'_ self) -> RWLockReadGuard<'_, T> {
        loop {
            let n = self.read_count_write_lock_flag.load(Ordering::Relaxed);
            // 書き込み、書き込み待ちフラグが立っていないかを検証し、立っていればspin
            if n & Self::WRITE_FLAG == 0
                && self
                    .read_count_write_lock_flag
                    .compare_exchange_weak(n, n + 1, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
            {
                return RWLockReadGuard { lock: self };
            }
            core::hint::spin_loop();
        }
    }

    pub fn write(&'_ self) -> RWLockWriteGuard<'_, T> {
        let n = self
            .read_count_write_lock_flag
            .fetch_or(Self::WRITE_FLAG, Ordering::Acquire);
        // 書き込みや読み込みがなされてないかを検証する
        if n == 0 {
            return RWLockWriteGuard { lock: self };
        }
        while self.read_count_write_lock_flag.load(Ordering::Relaxed) & !Self::WRITE_FLAG != 0 {
            // 失敗したらspin
            core::hint::spin_loop();
        }
        RWLockWriteGuard { lock: self }
    }
}

impl<T> Drop for RWLockReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock
            .read_count_write_lock_flag
            .fetch_sub(1, Ordering::Release);
    }
}

impl<T> Deref for RWLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> Drop for RWLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock
            .read_count_write_lock_flag
            .fetch_and(!RWLock::<T>::WRITE_FLAG, Ordering::Release);
    }
}

impl<T> Deref for RWLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for RWLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

#[cfg(test)]
mod tests {
    use core_affinity::CoreId;

    use super::*;
    extern crate core_affinity;
    use core::time::Duration;
    use std::sync::Arc;
    use std::thread;
    fn get_core_id() -> CoreId {
        let mut core_ids = core_affinity::get_core_ids().unwrap();
        core_ids.pop().unwrap()
    }
    #[test]
    fn spinlock_test() {
        let core_id = get_core_id();
        let test_data: Arc<SpinLock<usize>> = Arc::new(SpinLock::new(0));

        let handles: Vec<_> = (0..100)
            .map(|_| {
                let test_data_clone = Arc::clone(&test_data);
                thread::spawn(move || {
                    if core_affinity::set_for_current(core_id) {
                        for _ in 0..1000 {
                            let arc = test_data_clone.lock().lock;
                            let mut arc = arc.lock();
                            let data = arc.deref_mut();
                            *data += 1;
                        }
                    }
                })
            })
            .collect();

        for handle in handles.into_iter() {
            handle.join().unwrap();
        }
        assert_eq!(*test_data.lock().deref(), 100 * 1000);
    }

    #[test]
    fn rw_lock_data() {
        let core_id = get_core_id();
        let test_data: Arc<RWLock<usize>> = Arc::new(RWLock::new(0));

        let handles1: Vec<_> = (0..100)
            .map(|_| {
                let test_data_clone = Arc::clone(&test_data);
                thread::spawn(move || {
                    if core_affinity::set_for_current(core_id) {
                        assert_eq!(0, *test_data_clone.read().lock.read().deref());
                    }
                })
            })
            .collect();
        std::thread::sleep(Duration::from_millis(10));
        let handles2: Vec<_> = (0..100)
            .map(|_| {
                let test_data_clone = Arc::clone(&test_data);
                thread::spawn(move || {
                    if core_affinity::set_for_current(core_id) {
                        for _ in 0..1000 {
                            let arc = test_data_clone.write().lock;
                            let mut arc = arc.write();
                            let data = arc.deref_mut();
                            *data += 1;
                        }
                    }
                })
            })
            .collect();

        for handle in handles1.into_iter() {
            handle.join().unwrap();
        }
        for handle in handles2.into_iter() {
            handle.join().unwrap();
        }
        assert_eq!(*test_data.read().deref(), 100 * 1000);
    }
}
