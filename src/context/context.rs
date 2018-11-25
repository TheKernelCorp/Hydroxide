use alloc::{boxed::Box, vec, vec::Vec};

use crate::context::arch;
use crate::context::contexts;
use crate::context::memory::Memory;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Status {
    Runnable,
    Blocked,
    Stopped(usize),
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

#[derive(Debug)]
pub struct Context {
    pub ctx: arch::Context,
    pub stack: Box<[u8]>,

    func: *const (),

    pub status: Status,
    pub running: bool,

    pub id: usize,
}

impl Context {
    pub fn new<F>(id: usize, stack_size: usize, f: F) -> Context
    where
        F: FnOnce() + Send + Sync,
    {
        let stack = vec![0; stack_size].into_boxed_slice();

        Context {
            ctx: arch::Context::new(
                (stack.as_ptr() as usize + stack_size) as *mut _,
                common_context_entry::<F>,
            ),
            stack,

            func: Box::into_raw(Box::new(f)) as *const (),

            status: Status::Blocked,
            running: false,

            id,
        }
    }

    pub fn block(&mut self) -> bool {
        if self.status == Status::Runnable {
            self.status = Status::Blocked;
            true
        } else {
            false
        }
    }

    pub fn unblock(&mut self) -> bool {
        if self.status == Status::Blocked {
            self.status = Status::Runnable;
            true
        } else {
            false
        }
    }

    pub fn exit() {
        {
            let contexts = contexts();
            let mut current_context = contexts.current().unwrap().write();

            current_context.status = Status::Blocked;
        }

        unsafe {
            crate::context::switch::switch();
        }

        unreachable!();
    }
}

extern "C" fn common_context_entry<F>()
where
    F: FnOnce() + Send + Sync,
{
    {
        let contexts = contexts();
        let current_context = contexts.current().unwrap().read();

        let f = unsafe { Box::from_raw(current_context.func as *mut F) };
        f();
    }

    Context::exit();

    unreachable!();
}
