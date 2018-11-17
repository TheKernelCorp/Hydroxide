use spin::Mutex;
use x86_64::instructions::port::Port;
use lazy_static::lazy_static;

const KBC_DATA : u16 = 0x60;
const KBC_STATUS: u16 = 0x64;

//
// Keyboard I/O ports
//
lazy_static! {

    /// Data port
    pub static ref KBC_DATA_PORT: Mutex<Port<u8>> =
        Mutex::new(Port::new(KBC_DATA));

    /// Status port
    pub static ref KBC_STATUS_PORT: Mutex<Port<u8>> =
        Mutex::new(Port::new(KBC_STATUS));
}

// Keyboard Controller
pub struct KBC;
impl KBC {

    /// Wait for the KBC to become ready
    pub unsafe fn wait_ready() {
        KBC::with_ports(|data, status| {
            while status.read() & 0x02 != 0 {
                data.read(); // discard
            }
        });
    }

    /// Read the keyboard data port
    pub unsafe fn read_byte() -> u8 {
        KBC::wait_ready();
        KBC::with_ports(|data, _| data.read())
    }

    /// Write a byte to the keyboard data port
    pub unsafe fn write_byte(com: u8) {
        KBC::wait_ready();
        KBC::with_ports_mut(|data, _| data.write(com));
    }

    /// Provide a closure with read-only access to KBC ports
    pub fn with_ports<F, R>(f: F) -> R
        where F: Fn(&Port<u8>, &Port<u8>) -> R {

        // Lock ports
        let data_port = &*KBC_DATA_PORT.lock();
        let status_port = &*KBC_STATUS_PORT.lock();

        // Call closure
        f(data_port, status_port)
    }

    /// Provide a closure with read-write access to KBC ports
    pub fn with_ports_mut<F, R>(mut f: F) -> R
        where F: FnMut(&mut Port<u8>, &mut Port<u8>) -> R {

        // Lock ports
        let data_port = &mut *KBC_DATA_PORT.lock();
        let status_port = &mut *KBC_STATUS_PORT.lock();

        // Call closure
        f(data_port, status_port)
    }

    /// Reset the CPU
    pub unsafe fn reset_cpu() {
        KBC::wait_ready();

        // 0xF0 | 0x0E => Pulse line 0 for CPU reset
        KBC::with_ports_mut(|_, status| status.write(0xFE));
    }
}