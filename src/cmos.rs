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

/// POST status bit result
pub enum POSTResult {
    Ok,
    Fail,
    Yes,
    No,
}

impl core::fmt::Display for POSTResult {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let s = match *self {
            POSTResult::Ok => "OK",
            POSTResult::Fail => "FAIL",
            POSTResult::Yes => "YES",
            POSTResult::No => "NO",
        };
        write!(f, "{}", s)
    }
}

bitflags! {

    /// POST status data
    pub struct POSTData: u8 {
        const ADAPTER_TIMEOUT_CHECK = 0b_0000_0001;
        const ADAPTER_VALIDITY      = 0b_0000_0010;
        const TIME_VALIDITY         = 0b_0000_0100;
        const DRIVE_FAILURE         = 0b_0000_1000;
        const MEMORY_AMOUNT_MATCH   = 0b_0001_0000;
        const CONFIGURATION_MATCH   = 0b_0010_0000;
        const CMOS_CHECKSUM         = 0b_0100_0000;
        const POWER_SUPPLY          = 0b_1000_0000;
    }
}

macro_rules! impl_post_status {
    (fn $name:ident <- $testfor:expr, [0 => $cond_zero:expr, 1 => $cond_one:expr]) => {
        pub fn $name(&self) -> POSTResult {
            if self.contains($testfor) {
                $cond_one
            } else {
                $cond_zero
            }
        }
    };
}

impl POSTData {

    impl_post_status!(
        fn adapter_status <- Self::ADAPTER_TIMEOUT_CHECK, [
            0 => POSTResult::Ok,
            1 => POSTResult::Fail
        ]
    );

    impl_post_status!(
        fn adapter_init_status <- Self::ADAPTER_VALIDITY, [
            0 => POSTResult::Ok,
            1 => POSTResult::Fail
        ]
    );

    impl_post_status!(
        fn time_status <- Self::TIME_VALIDITY, [
            0 => POSTResult::Ok,
            1 => POSTResult::Fail
        ]
    );

    impl_post_status!(
        fn drive_status <- Self::DRIVE_FAILURE, [
            0 => POSTResult::Ok,
            1 => POSTResult::Fail
        ]
    );

    impl_post_status!(
        fn memory_match_status <- Self::MEMORY_AMOUNT_MATCH, [
            0 => POSTResult::Yes,
            1 => POSTResult::No
        ]
    );

    impl_post_status!(
        fn configuration_match_status <- Self::CONFIGURATION_MATCH, [
            0 => POSTResult::Yes,
            1 => POSTResult::No
        ]
    );

    impl_post_status!(
        fn cmos_checksum_status <- Self::CMOS_CHECKSUM, [
            0 => POSTResult::Ok,
            1 => POSTResult::Fail
        ]
    );

    impl_post_status!(
        fn power_supply_status <- Self::POWER_SUPPLY, [
            0 => POSTResult::Ok,
            1 => POSTResult::Fail
        ]
    );
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