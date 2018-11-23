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

#[derive(Debug, Default, Clone)]
pub struct CPUSnapshot {
    // 64-bit extended general-purpose registers
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    // 64-bit extended special-purpose registers
    rbp: u64, // base pointer
    rsp: u64, // stack pointer
    rsi: u64, // src index
    rdi: u64, // dst index
    // 64-bit dedicated registers
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
}

impl CPUSnapshot {
    #[naked]
    #[inline(never)]
    pub unsafe extern "C" fn create() -> CPUSnapshot {
        let mut snap = CPUSnapshot::default();
        asm!("mov $0, rax" : "=r"(snap.rax) : : "rax" : "intel", "volatile");
        asm!("mov $0, rbx" : "=r"(snap.rbx) : : "rbx" : "intel", "volatile");
        asm!("mov $0, rcx" : "=r"(snap.rcx) : : "rcx" : "intel", "volatile");
        asm!("mov $0, rdx" : "=r"(snap.rdx) : : "rdx" : "intel", "volatile");
        asm!("mov $0, rbp" : "=r"(snap.rbp) : : "rbp" : "intel", "volatile");
        asm!("mov $0, rsp" : "=r"(snap.rsp) : : "rsp" : "intel", "volatile");
        asm!("mov $0, rsi" : "=r"(snap.rsi) : : "rsi" : "intel", "volatile");
        asm!("mov $0, rdi" : "=r"(snap.rdi) : : "rdi" : "intel", "volatile");
        asm!("mov $0, r8"  : "=r"(snap.r8)  : : "r8"  : "intel", "volatile");
        asm!("mov $0, r9"  : "=r"(snap.r9)  : : "r9"  : "intel", "volatile");
        asm!("mov $0, r10" : "=r"(snap.r10) : : "r10" : "intel", "volatile");
        asm!("mov $0, r11" : "=r"(snap.r11) : : "r11" : "intel", "volatile");
        asm!("mov $0, r12" : "=r"(snap.r12) : : "r12" : "intel", "volatile");
        asm!("mov $0, r13" : "=r"(snap.r13) : : "r13" : "intel", "volatile");
        asm!("mov $0, r14" : "=r"(snap.r14) : : "r14" : "intel", "volatile");
        asm!("mov $0, r15" : "=r"(snap.r15) : : "r15" : "intel", "volatile");
        snap
    }

    #[naked]
    #[inline(never)]
    pub unsafe extern "C" fn apply(&self) {
        asm!("mov rax, $0" : : "r"(self.rax) : "rax" : "intel", "volatile");
        asm!("mov rbx, $0" : : "r"(self.rbx) : "rbx" : "intel", "volatile");
        asm!("mov rcx, $0" : : "r"(self.rcx) : "rcx" : "intel", "volatile");
        asm!("mov rdx, $0" : : "r"(self.rdx) : "rdx" : "intel", "volatile");
        asm!("mov rbp, $0" : : "r"(self.rbp) : "rbp" : "intel", "volatile");
        asm!("mov rsp, $0" : : "r"(self.rsp) : "rsp" : "intel", "volatile");
        asm!("mov rsi, $0" : : "r"(self.rsi) : "rsi" : "intel", "volatile");
        asm!("mov rdi, $0" : : "r"(self.rdi) : "rdi" : "intel", "volatile");
        asm!("mov r8,  $0" : : "r"(self.r8)  : "r8"  : "intel", "volatile");
        asm!("mov r9,  $0" : : "r"(self.r9)  : "r9"  : "intel", "volatile");
        asm!("mov r10, $0" : : "r"(self.r10) : "r10" : "intel", "volatile");
        asm!("mov r11, $0" : : "r"(self.r11) : "r11" : "intel", "volatile");
        asm!("mov r12, $0" : : "r"(self.r12) : "r12" : "intel", "volatile");
        asm!("mov r13, $0" : : "r"(self.r13) : "r13" : "intel", "volatile");
        asm!("mov r14, $0" : : "r"(self.r14) : "r14" : "intel", "volatile");
        asm!("mov r15, $0" : : "r"(self.r15) : "r15" : "intel", "volatile");
    }
}

#[derive(Debug, Clone)]
pub struct Context {
    pub rip: u64,
    rflags: u64,
    snapshot: CPUSnapshot,
}

impl Context {
    pub fn new(snapshot: CPUSnapshot) -> Self {
        Context {
            rflags: 0,
            rip: 0,
            snapshot,
        }
    }

    pub unsafe fn push_stack(&mut self, value: u64) {
        self.snapshot.rsp -= mem::size_of::<u64>() as u64;
        *(self.snapshot.rsp as *mut u64) = value;
    }

    pub fn set_stack(&mut self, address: u64) {
        self.snapshot.rsp = address;
    }

    #[cold]
    #[inline(never)]
    #[naked]
    pub unsafe fn switch_to(&mut self, next: &mut Context, stack_frame: &mut ExceptionStackFrame) {
        asm!("mov $0, rsp" : "=r"(self.snapshot.rsp) : : "rsp" : "intel", "volatile");
        self.rip = stack_frame.instruction_pointer.as_u64();
        next.push_stack(self.rip);
        stack_frame.instruction_pointer = VirtAddr::new(next.rip);
        stack_frame.stack_pointer = VirtAddr::new(next.snapshot.rsp);
        asm!("mov $0, rbp" : "=r"(self.snapshot.rbp) : : "rbp" : "intel", "volatile");
        asm!("mov rbp, $0" : : "r"(next.snapshot.rbp) : "rbp" : "intel", "volatile");
        self.rflags = rflags::read_raw();
        if next.rflags == 0 {
            next.rflags = self.rflags;
        }
        let mut next_rflags = RFlags::from_bits_truncate(next.rflags);
        next_rflags.set(RFlags::INTERRUPT_FLAG, true);
        next.rflags = next_rflags.bits();
        rflags::write_raw(next.rflags);
    }
}
