use crate::vgaterm::TerminalDevice;

use alloc::prelude::*;
use alloc::boxed::Box;
use lazy_static::lazy_static;

lazy_static! {
    static ref DEVICE_MANAGER: DeviceManager = DeviceManager {
        devices: Vec::new(),
    };
}

pub trait DeviceWrite<T> {
    fn write(&mut self, t: T);
}

pub trait Device {
    fn get_type(&self) -> DeviceType;
}

pub struct DeviceManager {
    devices: Vec<Box<Device + Send + Sync>>,
}

pub enum DeviceType {
    BlockDevice,
    CharDevice,
}
