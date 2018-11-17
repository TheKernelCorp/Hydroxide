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

    // Reset the CPU
    pub unsafe fn reset_cpu() {

        // Lock the data port
        let data_port = KBC_DATA_PORT.lock();

        // Lock the status port
        let mut status_port = KBC_STATUS_PORT.lock();

        // Wait for the KBC to become ready
        while status_port.read() & 0x02 != 0 {
            data_port.read(); // discard
        }

        // 0xF0 | 0x0E => Pulse line 0 for CPU reset
        status_port.write(0xFE);
    }
}