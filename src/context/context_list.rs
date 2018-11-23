use alloc::{collections::BTreeMap, sync::Arc, vec};
use core::mem;
use spin::RwLock;
use x86_64::registers::control::Cr3;

use super::context::Context;
use crate::paging::PAGING;

pub struct ContextList {
    map: BTreeMap<usize, Arc<RwLock<Context>>>,
    next_id: usize,
}

impl ContextList {
    pub fn new() -> Self {
        ContextList {
            map: BTreeMap::new(),
            next_id: 1,
        }
    }

    pub fn current(&self) -> Option<&Arc<RwLock<Context>>> {
        self.map.get(unsafe { &super::CONTEXT_ID })
    }

    pub fn iter(&self) -> ::alloc::collections::btree_map::Iter<usize, Arc<RwLock<Context>>> {
        self.map.iter()
    }

    pub fn new_context(&mut self) -> Result<&Arc<RwLock<Context>>, &str> {
        if self.next_id >= super::CONTEXT_MAX_CONTEXTS {
            self.next_id = 1;
        }

        while self.map.contains_key(&self.next_id) {
            self.next_id += 1;
        }

        if self.next_id >= super::CONTEXT_MAX_CONTEXTS {
            return Err("Could not create context");
        }

        let id = self.next_id;
        self.next_id += 1;

        assert!(self
            .map
            .insert(id, Arc::new(RwLock::new(Context::new(id))))
            .is_none());

        Ok(self
            .map
            .get(&id)
            .expect("Failed to insert new context. ID is out of bounds."))
    }

    pub fn spawn(&mut self, func: extern "C" fn()) -> Result<&Arc<RwLock<Context>>, &str> {
        let context_lock = self.new_context()?;
        let mut context = context_lock.write();
        let stack = vec![0; 65536].into_boxed_slice();
        let offset = stack.len() - mem::size_of::<usize>();
        let cr3 = Cr3::read();
        context
            .arch
            .set_stack((stack.as_ptr() as usize + offset) as u64);
        #[allow(clippy::fn_to_numeric_cast)]
        {
            context.arch.rip = func as u64;
        }
        context.kstack = Some(stack);
        Ok(context_lock)
    }
}
