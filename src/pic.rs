use pic8259_simple::ChainedPics;
use spin::Mutex;

//
// Constants
//

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

//
// Static PIC structure
//

static PICS: Mutex<ChainedPics> = Mutex::new(
    unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) }
);

//
// Exports
//

/// Intel 8259-compatible PIC
pub struct PIC8259;
impl PIC8259 {

    /// Remap the PIC
    pub fn init() {
        unsafe {
            PICS.lock().initialize();   
        }
    }

    /// Get the chained PICs.
    pub fn get_chained_pics() -> &'static Mutex<ChainedPics> {
        &PICS
    }
}