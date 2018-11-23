use core::mem;
use x86_64::{
    registers::{
        control::{Cr3, Cr3Flags},
        rflags,
        rflags::RFlags,
    },
    structures::{idt::ExceptionStackFrame, paging::PhysFrame},
    VirtAddr,
};

#[derive(Debug, Clone)]
pub struct Context {
    pub rip: u64,
    rflags: u64,
    rsp: u64,
    rbp: u64,
}

impl Context {
    pub fn new() -> Self {
        Context {
            rip: 0,
            rsp: 0,
            rbp: 0,
            rflags: 0,
        }
    }

    pub unsafe fn push_stack(&mut self, value: u64) {
        self.rsp -= mem::size_of::<u64>() as u64;
        *(self.rsp as *mut u64) = value;
    }

    pub fn set_stack(&mut self, address: u64) {
        self.rsp = address;
    }

    #[cold]
    #[inline(never)]
    #[naked]
    pub unsafe fn switch_to(&mut self, next: &mut Context, stack_frame: &mut ExceptionStackFrame) {
        asm!("mov $0, rsp" : "=r"(self.rsp) : : "memory" : "intel", "volatile");
        self.rip = stack_frame.instruction_pointer.as_u64();
        next.push_stack(self.rip);
        stack_frame.instruction_pointer = VirtAddr::new(next.rip);
        stack_frame.stack_pointer = VirtAddr::new(next.rsp);
        asm!("mov $0, rbp" : "=r"(self.rbp) : : "memory" : "intel", "volatile");
        asm!("mov rbp, $0" : : "r"(next.rbp) : "memory" : "intel", "volatile");
        self.rflags = rflags::read_raw();
        if next.rflags == 0 {
            next.rflags = self.rflags;
        }
        rflags::write_raw(next.rflags);
    }
}
