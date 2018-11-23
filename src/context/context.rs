use alloc::boxed::Box;

use crate::context::arch;
use crate::context::memory::Memory;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Status {
    Runnable,
    Blocked,
    Stopped(usize),
}

#[derive(Debug)]
pub struct Context {
    pub id: usize,
    pub status: Status,
    pub running: bool,
    pub arch: arch::Context,
    pub kstack: Option<Box<[u8]>>,
    pub stack: Option<Memory>,
}

impl Context {
    pub fn new(id: usize) -> Context {
        Context {
            id,
            status: Status::Blocked,
            running: false,
            arch: arch::Context::new(arch::CPUSnapshot::default()),
            kstack: None,
            stack: None,
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
}
