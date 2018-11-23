use x86_64::instructions::interrupts;
use x86_64::registers::rflags::RFlags;
use x86_64::structures::idt::ExceptionStackFrame;

use super::context::{Context, Status};
use super::{contexts, CONTEXT_ID};

unsafe fn runnable(context: &Context) -> bool {
    !context.running && context.status == Status::Runnable
}

pub unsafe fn switch(stack_frame: &mut ExceptionStackFrame) -> bool {
    use core::ops::DerefMut;

    let from_ptr;
    let mut to_ptr: *mut Context = core::ptr::null_mut::<Context>();
    let contexts = contexts();
    {
        let context_lock = contexts
            .current()
            .expect("context::switch: not inside of context");
        let mut context = context_lock.write();
        from_ptr = context.deref_mut() as *mut Context;
    }

    for (pid, context_lock) in contexts.iter() {
        if *pid > (*from_ptr).id {
            let mut context = context_lock.write();
            if runnable(&context) {
                to_ptr = context.deref_mut() as *mut Context;
                break;
            }
        }
    }

    if to_ptr as usize == 0 {
        for (pid, context_lock) in contexts.iter() {
            if *pid < (*from_ptr).id {
                let mut context = context_lock.write();
                if runnable(&context) {
                    to_ptr = context.deref_mut() as *mut Context;
                    break;
                }
            }
        }
    }

    if to_ptr as usize != 0 {
        #[allow(clippy::ref_in_deref)]
        {
            (&mut *from_ptr).running = false;
            (&mut *to_ptr).running = true;
        }
        CONTEXT_ID = (*to_ptr).id;
    }

    if to_ptr as usize == 0 {
        false
    } else {
        (&mut *from_ptr)
            .arch
            .switch_to(&mut (&mut *to_ptr).arch, stack_frame);

        true
    }
}
