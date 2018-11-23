use crate::pic::PIC8259;
use x86_64::structures::idt::ExceptionStackFrame;
use x86_64::VirtAddr;

use crate::context::switch;

pub extern "x86-interrupt" fn handle_interrupt(stack_frame: &mut ExceptionStackFrame) {
    unsafe {
        print!("b");
        switch::switch(stack_frame);

        PIC8259::get_chained_pics()
            .lock()
            .notify_end_of_interrupt(crate::idt::INT_PIT);
    }
}
