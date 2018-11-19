use alloc::prelude::*;
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use lazy_static::lazy_static;

use core::any::Any;

use spin::Mutex;

lazy_static! {
    pub static ref DEVICE_MANAGER: Mutex<DeviceManager> = Mutex::new(DeviceManager {
        devices: BTreeMap::new(),
    });
}

pub trait Device {
    fn get_type(&self) -> DeviceType;

    fn write_byte(&mut self, at: usize, val: u8);
    fn write_bytes(&mut self, at: usize, val: &[u8], len: usize);

    fn as_any(&self) -> &dyn Any;
}

pub struct DeviceManager {
    devices: BTreeMap<&'static str, Box<dyn Device + Sync + Send>>,
}

impl DeviceManager {
    pub fn register_device(&mut self, name: &'static str, dev: Box<dyn Device + Sync + Send>) -> Result<(), String> {
        if self.devices.contains_key(name) {
            return Err(format!("Device {} already registered.", name));
        }
        self.devices.insert(name, dev);
        Ok(())
    }

    pub fn get_device(&self, name: &'static str) -> Option<&Box<dyn Device + Sync + Send>> {
        let dev = self.devices.get(name).unwrap();
        Some(dev)
    }
}

pub enum DeviceType {
    BlockDevice,
    CharDevice,
}
