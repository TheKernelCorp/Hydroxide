use alloc::{boxed::Box, vec, vec::Vec};
use core::ptr;

use super::{atomic::{Atomic, Ordering}, mpsc::IntrusiveNode};
use crate::arch::x86_64::{context::Context, cpu::Local};

impl IntrusiveNode for Thread {
    #[inline]
    unsafe fn get_next(self: *mut Thread) -> *mut Thread {
        (*self).next_thread
    }

    #[inline]
    unsafe fn set_next(self: *mut Thread, next: *mut Thread) {
        (*self).next_thread = next;
    }

    #[inline]
    unsafe fn is_on_queue(self: *mut Thread) -> bool {
        !(*self).next_thread.is_null()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum State {
    Initial,
    Ready,
    Running,
    Suspended,
    Blocked,
    Killable,
    Dead,
}

pub struct Thread {
    pub ctx: Context,
    pub stack: Box<[u8]>,
    func: *const (),
    next_thread: *mut Thread,
    state: Atomic<State>,
}

unsafe impl Send for Thread {}
unsafe impl Sync for Thread {}

impl Thread {
    pub fn new<F>(stack_size: usize, f: F) -> Result<Box<Thread>, &'static str>
    where
        F: FnOnce() + Send + Sync,
    {
        let stack = vec![0; stack_size].into_boxed_slice();

        Ok(Box::new(Thread {
            ctx: Context::new(
                (stack.as_ptr() as usize + stack_size) as *mut _,
                common_thread_entry::<F>,
            ),
            stack,
            func: Box::into_raw(box f) as *const (),
            next_thread: ptr::null_mut(),
            state: Atomic::new(State::Initial),
        }))
    }

    pub fn start(&mut self) {
        let old_state = self
            .state
            .compare_and_swap(State::Initial, State::Ready, Ordering::SeqCst);

        assert_eq!(old_state, State::Initial);

        Local::schedule_thread(self);
    }

    pub fn state(&self) -> State {
        self.state.load(Ordering::Relaxed)
    }

    pub fn set_state(&self, state: State) {
        self.state.store(state, Ordering::Relaxed);
    }

    pub fn current<'a>() -> &'a mut Thread {
        unsafe { &mut *Local::current_thread() }
    }

    pub fn yield_now() {
        unsafe {
            Local::context_switch();
        }
    }

    pub fn resume(&self) {
        assert!({
            let state = self.state();
            state == State::Blocked || state == State::Suspended
        });

        self.set_state(State::Ready);

        Local::schedule_thread(self as *const _ as *mut _);
    }

    pub fn join(self: Box<Self>) -> Result<(), &'static str> {
        if &*self as *const _ == Thread::current() as *const _ {
            return Err("Cannot join with the current thread");
        }

        self.set_state(State::Dead);

        Ok(())
    }

    pub fn kill(self: Box<Self>) {
        if &*self as *const _ == Thread::current() as *const _ {
            return;
        }

        if !self.next_thread.is_null() {
            self.set_state(State::Killable);

            Box::into_raw(self);
        }
    }

    pub fn exit() {
        let current_thread = Thread::current();

        assert!(current_thread.next_thread.is_null());

        current_thread.set_state(State::Dead);

        unsafe {
            Local::context_switch();
        }

        unreachable!();
    }
}

extern "C" fn common_thread_entry<F>()
where
    F: FnOnce() + Send + Sync,
{
    let current_thread = Thread::current();

    let f = unsafe { Box::from_raw(current_thread.func as *mut F) };
    f();

    Thread::exit();

    unreachable!();
}
