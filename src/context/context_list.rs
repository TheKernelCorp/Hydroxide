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
            next_id: 0,
        }
    }

    pub fn current(&self) -> Option<&Arc<RwLock<Context>>> {
        self.map.get(unsafe { &super::CONTEXT_ID })
    }

    pub fn iter(&self) -> ::alloc::collections::btree_map::Iter<usize, Arc<RwLock<Context>>> {
        self.map.iter()
    }

    pub fn new_context<F>(&mut self, stack_size: usize, f: F) -> Result<&Arc<RwLock<Context>>, &str>
    where
        F: FnOnce() + Send + Sync,
    {
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
            .insert(id, Arc::new(RwLock::new(Context::new(id, stack_size, f))))
            .is_none());

        Ok(self
            .map
            .get(&id)
            .expect("Failed to insert new context. ID is out of bounds."))
    }
}
