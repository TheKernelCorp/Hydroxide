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
    fn write(&mut self, at: usize, t: T);
}

pub trait DeviceRead<T> {
    fn read(&mut self, at: usize) -> T;
}

pub trait Device {
    fn get_type(&self) -> DeviceType;

    fn as_write<T>(&self) -> Result<Box<DeviceWrite<T>>, &'static str>;
}

pub struct DeviceManager {
    devices: Vec<Box<dyn Device + Send + Sync>>,
}

pub enum DeviceType {
    BlockDevice,
    CharDevice,
}
