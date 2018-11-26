use alloc::boxed::Box;
use core::ptr::NonNull;

use x86_64::registers::model_specific::Msr;

use super::asm::*;
use crate::context::{
    scheduler::Scheduler,
    thread::{State, Thread},
};

pub type CpuID = u32;

pub struct Cpu {
    cpu_id: CpuID,
}

impl Cpu {
    pub fn id(&self) -> CpuID {
        self.cpu_id
    }
}

pub unsafe fn init(cpu_id: u32) {
    let cpu = Box::new(Cpu { cpu_id });

    let mut cpu_local = box Local::new(Box::leak(cpu));

    cpu_local.direct = (&*cpu_local).into();

    Msr::new(0xC0000101).write(Box::into_raw(cpu_local) as u64);
}

pub struct Local {
    direct: NonNull<Local>,
    _cpu: *const Cpu,

    scheduler: Scheduler,
    pub current_thread: *mut Thread,
}

impl Local {
    unsafe fn new(cpu: *const Cpu) -> Self {
        let idle_thread = Thread::new(4096, || loop {
            x86_64::instructions::hlt();
        })
        .unwrap();

        let kernel_thread = Thread::new(4096, || {}).unwrap();

        idle_thread.set_state(State::Ready);
        kernel_thread.set_state(State::Dead);

        Local {
            direct: NonNull::dangling(),
            _cpu: cpu,
            scheduler: Scheduler::new(Box::into_raw(idle_thread)),
            current_thread: Box::into_raw(kernel_thread),
        }
    }

    pub fn current() -> &'static mut Local {
        unsafe { &mut *(read_gs_offset64!(0x0) as *mut Local) }
    }

    #[inline]
    pub fn current_thread() -> *mut Thread {
        unsafe { read_gs_offset64!(offset_of!(Local, current_thread)) as *mut Thread }
    }

    #[inline]
    pub fn set_current_thread(thread: *mut Thread) {
        unsafe {
            asm!("mov $0, %gs:0x28" : : "r"(thread) : "memory" : "volatile");
        }
    }

    pub fn schedule_thread(thread: *mut Thread) {
        Self::current().scheduler.schedule_thread(thread);
    }

    pub unsafe fn context_switch() {
        Self::current().scheduler.switch();
    }
}
