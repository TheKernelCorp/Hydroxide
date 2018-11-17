use spin::Mutex;
use x86_64::instructions::port::Port;
use lazy_static::lazy_static;
use bitflags::bitflags;

const CMOS_ADDR: u16 = 0x70;
const CMOS_DATA: u16 = 0x71;

lazy_static! {
    static ref CMOS_PORT_ADDR: Mutex<Port<u8>> = Mutex::new(Port::new(CMOS_ADDR));
    static ref CMOS_PORT_DATA: Mutex<Port<u8>> = Mutex::new(Port::new(CMOS_DATA));
}

bitflags! {
    pub struct POSTData: u8 {
        const ADAPTER_TIMEOUT_CHECK = 0b00000001;
        const ADAPTER_VALIDITY      = 0b00000010;
        const TIME_VALIDITY         = 0b00000100;
        const DRIVE_FAILURE         = 0b00001000;
        const MEMORY_AMOUNT_MATCH   = 0b00010000;
        const CONFIGURATION_MATCH   = 0b00100000;
        const CMOS_CHECKSUM         = 0b01000000;
        const POWER_SUPPLY          = 0b10000000;
    }
}

pub struct CMOS;
impl CMOS {

    pub fn read_post_data() -> Option<POSTData> {
        let b = unsafe { CMOS::read(0x0E) };
        POSTData::from_bits(b)
    }

    /// Read a byte from the CMOS
    pub unsafe fn read(offset: u8) -> u8 {
        CMOS::with_ports_mut(|addr, data| {
            let tmp = addr.read();
            addr.write((tmp & 0x80) | (offset & 0x7F));
            data.read()
        })
    }

    /// Write a byte to the CMOS
    pub unsafe fn write(offset: u8, value: u8) {
        CMOS::with_ports_mut(|addr, data| {
            let tmp = addr.read();
            addr.write((tmp & 0x80) | (offset & 0x7F));
            data.write(value);
        });
    }

    /// Provide a closure with read-only access to CMOS ports
    pub fn with_ports<F, R>(f: F) -> R
        where F: Fn(&Port<u8>, &Port<u8>) -> R {

        // Lock ports
        let addr_port = &*CMOS_PORT_ADDR.lock();
        let data_port = &*CMOS_PORT_DATA.lock();

        // Call closure
        f(addr_port, data_port)
    }

    /// Provide a closure with read-write access to CMOS ports
    pub fn with_ports_mut<F, R>(mut f: F) -> R
        where F: FnMut(&mut Port<u8>, &mut Port<u8>) -> R {

        // Lock ports
        let addr_port = &mut *CMOS_PORT_ADDR.lock();
        let data_port = &mut *CMOS_PORT_DATA.lock();

        // Call closure
        f(addr_port, data_port)
    }
}