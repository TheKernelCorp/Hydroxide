use super::interrupts;
use crate::context::atomic::{Atomic, Ordering};

use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut, Drop},
};

#[derive(Debug)]
pub struct Spinlock<T: ?Sized> {
    lock: Atomic<bool>,
    data: UnsafeCell<T>,
}

pub struct SpinGuard<'a, T: ?Sized + 'a> {
    lock: &'a Atomic<bool>,
    data: &'a mut T,
}

unsafe impl<T: ?Sized + Send> Sync for Spinlock<T> {}
unsafe impl<T: ?Sized + Send> Send for Spinlock<T> {}

impl<T> Spinlock<T> {
    pub const fn new(data: T) -> Spinlock<T> {
        Spinlock {
            lock: Atomic::new(false),
            data: UnsafeCell::new(data),
        }
    }

    fn obtain_lock(&self) {
        while self.lock.compare_and_swap(false, true, Ordering::Acquire) != false {
            while self.lock.load(Ordering::Relaxed) {
                interrupts::pause();
            }
        }
    }

    pub fn lock(&self) -> SpinGuard<T> {
        self.obtain_lock();

        SpinGuard {
            lock: &self.lock,
            data: unsafe { &mut *self.data.get() },
        }
    }

    pub fn try_lock(&self) -> Option<SpinGuard<T>> {
        if self.lock.compare_and_swap(false, true, Ordering::Acquire) == false {
            Some(SpinGuard {
                lock: &self.lock,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            None
        }
    }

    pub fn held(&self) -> bool {
        self.lock.load(Ordering::Relaxed)
    }
}

impl Spinlock<()> {
    pub fn lock_unguarded(&self) {
        self.obtain_lock();
    }

    pub fn unlock_unguarded(&self) {
        self.lock.store(false, Ordering::Release);
    }
}

impl<T: ?Sized + Default> Default for Spinlock<T> {
    fn default() -> Spinlock<T> {
        Spinlock::new(Default::default())
    }
}

impl<'a, T: ?Sized> SpinGuard<'a, T> {
    pub fn release(self) {}
}

impl<'a, T: ?Sized> Deref for SpinGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &*self.data
    }
}

impl<'a, T: ?Sized> DerefMut for SpinGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.data
    }
}

impl<'a, T: ?Sized> Drop for SpinGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
    }
}

pub struct IrqLock<T: ?Sized> {
    data: UnsafeCell<T>,
}

pub struct IrqGuard<'a, T: ?Sized + 'a> {
    data: &'a mut T,
    was_enabled: bool,
}

unsafe impl<T: ?Sized + Send> Sync for IrqLock<T> {}
unsafe impl<T: ?Sized + Send> Send for IrqLock<T> {}

impl<T> IrqLock<T> {
    pub const fn new(data: T) -> IrqLock<T> {
        IrqLock {
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> IrqGuard<T> {
        let was_enabled = x86_64::instructions::interrupts::are_enabled();
        if was_enabled {
            unsafe {
                interrupts::disable();
            }
        }

        IrqGuard {
            data: unsafe { &mut *self.data.get() },
            was_enabled,
        }
    }

    pub fn lock_map<F, U>(&self, f: F) -> IrqGuard<U>
    where
        F: FnOnce(&mut T) -> &mut U,
    {
        let was_enabled = x86_64::instructions::interrupts::are_enabled();
        if was_enabled {
            unsafe {
                interrupts::disable();
            }
        }

        let data = f(unsafe { &mut *self.data.get() });

        IrqGuard { data, was_enabled }
    }
}

impl<'a, T: ?Sized> IrqGuard<'a, T> {
    pub fn release(self) {}
}

impl<'a, T: ?Sized> Deref for IrqGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &*self.data
    }
}

impl<'a, T: ?Sized> DerefMut for IrqGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.data
    }
}

impl<'a, T: ?Sized> Drop for IrqGuard<'a, T> {
    fn drop(&mut self) {
        if self.was_enabled {
            unsafe {
                interrupts::enable();
            }
        }
    }
}

#[derive(Debug)]
pub struct IrqSpinlock<T: ?Sized> {
    lock: Atomic<bool>,
    data: UnsafeCell<T>,
}

pub struct IrqSpinGuard<'a, T: ?Sized + 'a> {
    lock: &'a Atomic<bool>,
    was_enabled: bool,
    data: &'a mut T,
}

unsafe impl<T: ?Sized + Send> Sync for IrqSpinlock<T> {}
unsafe impl<T: ?Sized + Send> Send for IrqSpinlock<T> {}

impl<T> IrqSpinlock<T> {
    pub const fn new(data: T) -> IrqSpinlock<T> {
        IrqSpinlock {
            lock: Atomic::new(false),
            data: UnsafeCell::new(data),
        }
    }

    fn obtain_lock(&self) {
        while self.lock.compare_and_swap(false, true, Ordering::Acquire) != false {
            while self.lock.load(Ordering::Release) {
                interrupts::pause();
            }
        }
    }

    pub fn lock(&self) -> IrqSpinGuard<T> {
        self.obtain_lock();

        let was_enabled = x86_64::instructions::interrupts::are_enabled();
        if was_enabled {
            unsafe {
                interrupts::disable();
            }
        }

        IrqSpinGuard {
            lock: &self.lock,
            was_enabled,
            data: unsafe { &mut *self.data.get() },
        }
    }

    pub fn try_lock(&self) -> Option<IrqSpinGuard<T>> {
        if self.lock.compare_and_swap(false, true, Ordering::Acquire) == false {
            let was_enabled = x86_64::instructions::interrupts::are_enabled();
            if was_enabled {
                unsafe {
                    interrupts::disable();
                }
            }
            Some(IrqSpinGuard {
                lock: &self.lock,
                was_enabled,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            None
        }
    }
}

impl<T: ?Sized + Default> Default for IrqSpinlock<T> {
    fn default() -> IrqSpinlock<T> {
        IrqSpinlock::new(Default::default())
    }
}

impl<'a, T: ?Sized> IrqSpinGuard<'a, T> {
    pub fn release(self) {}
}

impl<'a, T: ?Sized> Deref for IrqSpinGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &*self.data
    }
}

impl<'a, T: ?Sized> DerefMut for IrqSpinGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.data
    }
}

impl<'a, T: ?Sized> Drop for IrqSpinGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
        if self.was_enabled {
            unsafe {
                interrupts::enable();
            }
        }
    }
}
