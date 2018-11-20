use core::any::Any;
use x86_64::instructions::port::Port;
use crate::hal::{Device, DeviceType, DEVICE_MANAGER};

pub enum SerialPort {
    COM1 = 0x3F8,
    COM2 = 0x2F8,
    COM3 = 0x3E8,
    COM4 = 0x2E8,
}

pub struct SerialDevice {
    data: Port<u8>,
    dlab_lo: Port<u8>,
    dlab_hi: Port<u8>,
    int_ctrl: Port<u8>,
    fifo_ctrl: Port<u8>,
    line_ctrl: Port<u8>,
    modem_ctrl: Port<u8>,
    line_status: Port<u8>,
    modem_status: Port<u8>,
    scratch: Port<u8>,
}

impl SerialDevice {

    pub fn init(name: &'static str, port: SerialPort) -> Result<(), &str> {
        let base_port = port as u16;

        // Create the serial device
        let mut dev = SerialDevice {
            data: Port::new(base_port),
            dlab_lo: Port::new(base_port),
            int_ctrl: Port::new(base_port + 1),
            dlab_hi: Port::new(base_port + 1),
            fifo_ctrl: Port::new(base_port + 2),
            line_ctrl: Port::new(base_port + 3),
            modem_ctrl: Port::new(base_port + 4),
            line_status: Port::new(base_port + 5),
            modem_status: Port::new(base_port + 6),
            scratch: Port::new(base_port + 7),
        };

        // Initialzie the serial device
        unsafe { dev.init_bus() }

        // Register the device
        DEVICE_MANAGER.lock().register_device(name, box dev).expect("Unable to initialize serial device!");
        Ok(())
    }

    unsafe fn init_bus(&mut self) {
        self.int_ctrl.write(0x00); // Disable INTs
        self.line_ctrl.write(0x80); // Enable DLAB
        self.dlab_lo.write(0x03); // Set divisor lo byte
        self.dlab_hi.write(0x00); // Set divisor hi byte
        self.line_ctrl.write(0x03); // 8 bits, no parity, 1 stop bit
        self.fifo_ctrl.write(0xC7); // Enable FIFO, clear with 14-byte threshold
        self.modem_ctrl.write(0x0B); // Enable IRQs, set RTS/DSR
    }

    unsafe fn is_empty(&self) -> bool {
        self.line_status.read() & 0x20 != 0
    }

    unsafe fn has_received(&self) -> bool {
        self.line_status.read() & 0x1 != 0
    }

    unsafe fn read_u8_block(&self) -> u8 {
        while !self.has_received() {}
        self.data.read()
    }

    unsafe fn read_u8_now(&self) -> Option<u8> {
        if self.has_received() {
            Some(self.data.read())
        } else {
            None
        }
    }

    unsafe fn write_u8_block(&mut self, val: u8) {
        while !self.is_empty() {}
        self.data.write(val)
    }

    unsafe fn write_u8_now(&mut self, val: u8) -> Option<()> {
        if self.is_empty() {
            self.data.write(val);
            Some(())
        } else {
            None
        }
    }
}

impl Device for SerialDevice {

    fn get_type(&self) -> DeviceType {
        DeviceType::CharDevice
    }

    fn write_byte(&mut self, at: usize, val: u8) {
        unsafe { self.write_u8_block(val) }
    }

    fn write_bytes(&mut self, at: usize, val: &[u8], len: usize) {
        for b in val {
            self.write_byte(0, *b);
        }
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

unsafe impl Sync for SerialDevice {}
unsafe impl Send for SerialDevice {}

impl core::fmt::Write for SerialDevice {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let len = s.len();
        self.write_bytes(0, bytes, len);
        Ok(())
    }
}