use alloc::{boxed::Box, vec};
use core::mem;
use core::slice;
use spin::{Once, RwLock, RwLockReadGuard, RwLockWriteGuard};

#[path = "arch/x86_64.rs"]
mod arch;

mod memory;

pub mod switch;

pub mod context;
use self::context::Status;

mod context_list;
use self::context_list::ContextList;

use crate::paging::PAGING;

pub const CONTEXT_MAX_CONTEXTS: usize = (isize::max_value() as usize) - 1;

static CONTEXTS: Once<RwLock<ContextList>> = Once::new();

pub static mut CONTEXT_ID: usize = 0;

fn init_contexts() -> RwLock<ContextList> {
    RwLock::new(ContextList::new())
}

pub fn contexts() -> RwLockReadGuard<'static, ContextList> {
    CONTEXTS.call_once(init_contexts).read()
}

pub fn contexts_mut() -> RwLockWriteGuard<'static, ContextList> {
    CONTEXTS.call_once(init_contexts).write()
}
