use lazy_static::lazy_static;
use x86_64::structures::idt::{ExceptionStackFrame, InterruptDescriptorTable, PageFaultErrorCode};

//
// Constants
//

/// PIT825x interrupt code
pub const INT_PIT: u8 = crate::pic::PIC_1_OFFSET;

/// PS/2 Keyboard interrupt code
pub const INT_KBD: u8 = crate::pic::PIC_1_OFFSET + 1;

lazy_static! {
    static ref STATIC_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.page_fault.set_handler_fn(page_fault_handler);
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(crate::gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt[usize::from(INT_PIT)].set_handler_fn(crate::pit::handle_interrupt);
        idt[usize::from(INT_KBD)].set_handler_fn(crate::ps2kbd::handle_interrupt);
        idt
    };
}

/// Interrupt Descriptor Table
pub struct IDT;
impl IDT {
    /// Initialize the IDT
    pub fn init() {
        // Load the IDT
        STATIC_IDT.load();
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut ExceptionStackFrame) {
    log!(fault: "*** BREAKPOINT EXCEPTION\r\n{:#?}", stack_frame);
    loop {}
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: &mut ExceptionStackFrame,
    error: PageFaultErrorCode,
) {
    log!(fault: "*** PAGE FAULT\r\n{:#?}\r\n{:#?}", stack_frame, error);
    loop {}
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut ExceptionStackFrame,
    error_code: u64,
) {
    log!(
        fault:
        "*** DOUBLE FAULT EXCEPTION\r\n{:#?}\r\nCODE: {:x}",
        stack_frame, error_code
    );
    loop {}
}
