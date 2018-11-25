use core::{cell::Cell, ptr};

use super::atomic::{Atomic, Ordering};

pub trait IntrusiveNode {
    unsafe fn get_next(self: *mut Self) -> *mut Self;
    unsafe fn set_next(self: *mut Self, next: *mut Self);
    unsafe fn is_on_queue(self: *mut Self) -> bool;
}

unsafe impl<T: IntrusiveNode> Sync for IntrusiveMpsc<T> {}
unsafe impl<T: IntrusiveNode> Send for IntrusiveMpsc<T> {}

pub struct IntrusiveMpsc<T: IntrusiveNode> {
    pushlist: Atomic<*mut T>,
    poplist: Cell<*mut T>,
}

impl<T: IntrusiveNode> IntrusiveMpsc<T> {
    pub const fn new() -> IntrusiveMpsc<T> {
        IntrusiveMpsc {
            pushlist: Atomic::new(ptr::null_mut()),
            poplist: Cell::new(ptr::null_mut()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.poplist.get().is_null() && self.pushlist.load(Ordering::Relaxed).is_null()
    }

    #[inline]
    pub unsafe fn push(&self, item_ptr: *mut T) {
        assert!(!item_ptr.is_on_queue());
        let old_head = self.pushlist.load(Ordering::Relaxed);

        item_ptr.set_next(old_head);

        while self
            .pushlist
            .compare_exchange_weak(old_head, item_ptr, Ordering::Release, Ordering::Relaxed)
            .is_err()
        {}
    }

    #[inline]
    pub unsafe fn pop(&self) -> Option<*mut T> {
        if !self.poplist.get().is_null() {
            let intrusive_node = self.poplist.get();
            self.poplist.set(intrusive_node.get_next());
            intrusive_node.set_next(ptr::null_mut());
            Some(intrusive_node)
        } else {
            let mut intrusive_node = self.pushlist.swap(ptr::null_mut(), Ordering::Acquire);
            if intrusive_node.is_null() {
                return None;
            }

            while !intrusive_node.get_next().is_null() {
                let next = intrusive_node.get_next();
                intrusive_node.set_next(self.poplist.get());
                self.poplist.set(intrusive_node);
                intrusive_node = next;
            }

            assert!(!intrusive_node.is_on_queue());

            Some(intrusive_node)
        }
    }
}
