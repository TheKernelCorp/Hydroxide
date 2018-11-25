use core::mem;
use x86_64::registers::rflags::RFlags;

global_asm!(include_str!("switch.asm"));

extern "C" {
    fn x86_64_context_switch(prev: *mut Context, next: *const Context);
}

#[repr(C)]
#[derive(Debug)]
pub struct Context {
    rflags: u64,
    rbx: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    rbp: u64,
    rsp: u64,
}

impl Context {
    pub fn new(stack_top: *mut u8, entry: extern "C" fn()) -> Self {
        let mut ctx = Context {
            rflags: RFlags::INTERRUPT_FLAG.bits(),
            rbx: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rbp: stack_top as _,
            rsp: stack_top as _,
        };

        unsafe {
            ctx.push_stack(entry as _);
        }

        ctx
    }

    pub unsafe fn push_stack(&mut self, item: usize) {
        self.rsp -= mem::size_of::<usize>() as u64;
        *(self.rsp as *mut usize) = item;
    }

    #[inline]
    pub unsafe fn swap(&mut self, next: &Context) {
        x86_64_context_switch(self as *mut _, next as *const _);
    }
}
