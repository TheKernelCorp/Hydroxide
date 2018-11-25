use crate::pic::PIC8259;
use x86_64::instructions::port::Port;
use x86_64::structures::idt::ExceptionStackFrame;
use x86_64::VirtAddr;

use crate::arch::cpu::Local;

static mut PIT_TICKS: usize = 0;

const PIT_A: u16 = 0x40;
const PIT_CTRL: u16 = 0x43;

const PIT_SCALE: usize = 1193180;
const PIT_SET: u8 = 0x36;

pub unsafe fn set_frequency(hz: usize) {
    let divisor = PIT_SCALE / hz;
    let mut pit_ctrl = Port::new(PIT_CTRL);
    let mut pit_a = Port::new(PIT_A);
    pit_ctrl.write(PIT_SET);
    pit_a.write((divisor as u8));
    pit_a.write((divisor >> 8) as u8);
}

pub extern "x86-interrupt" fn handle_interrupt(stack_frame: &mut ExceptionStackFrame) {
    unsafe {
        // Signal EOI
        PIC8259::get_chained_pics()
            .lock()
            .notify_end_of_interrupt(crate::idt::INT_PIT);

        PIT_TICKS += 1;

        // Do the context switch
        if PIT_TICKS >= 10 {
            Local::context_switch();
            PIT_TICKS = 0;
        }
    }
}
