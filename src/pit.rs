use x86_64::structures::idt::ExceptionStackFrame;
use crate::pic::PIC8259;

pub extern "x86-interrupt" fn handle_interrupt(_stack_frame: &mut ExceptionStackFrame) {
    unsafe {
        PIC8259
            ::get_chained_pics()
            .lock()
            .notify_end_of_interrupt(crate::idt::INT_PIT);
    }
}