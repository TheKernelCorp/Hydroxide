use alloc::prelude::*;
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use core::ptr::NonNull;
use core::cell::RefCell;
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

    fn as_any(&mut self) -> &mut dyn Any;
}

pub struct DeviceManager {
    devices: BTreeMap<&'static str, Mutex<Box<dyn Device + Sync + Send>>>,
}

impl DeviceManager {
    pub fn register_device(&mut self, name: &'static str, dev: Box<dyn Device + Sync + Send>) -> Result<(), String> {
        if self.devices.contains_key(name) {
            return Err(format!("Device {} already registered.", name));
        }
        self.devices.insert(name, Mutex::new(dev));
        Ok(())
    }

    pub fn get_device(&mut self, name: &'static str) -> Option<&Mutex<Box<dyn Device + Sync + Send>>> {
        let dev = self.devices.get(name).unwrap();
        Some(dev)
    }

    pub fn with_device_cast<T, D: 'static>(&mut self, dev: &str, f: T) where T: Fn(&mut D) {
        let mut boxed = self.devices.get(dev).unwrap().lock();
        let dev = boxed.as_any().downcast_mut::<D>().unwrap();
        f(dev);
    }
}

unsafe impl Send for DeviceManager {}

pub enum DeviceType {
    BlockDevice,
    CharDevice,
}
