#![allow(dead_code)]

use crate::kbc::KBC;
use crate::pic::PIC8259;
use lazy_static::lazy_static;
use pc_keyboard::{layouts::Us104Key, DecodedKey, Keyboard, ScancodeSet1};
use spin::Mutex;
use x86_64::structures::idt::ExceptionStackFrame;

//
// Global state
//

lazy_static! {
    pub static ref KEYBOARD: Mutex<Keyboard<Us104Key, ScancodeSet1>> =
        Mutex::new(Keyboard::new(Us104Key, ScancodeSet1));
    pub static ref KEYBOARD_INITIALIZED: Mutex<bool> = Mutex::new(false);
}

//
// Keyboard responses
//
// Terminology:
// ACK = Acknowledge
// ST = Self test
//

const KBD_RES_ACK: u8 = 0xFA;
const KBD_RES_ECHO: u8 = 0xEE;
const KBD_RES_RESEND: u8 = 0xFE;
const KBD_RES_ERROR_A: u8 = 0x00;
const KBD_RES_ERROR_B: u8 = 0xFF;
const KBD_RES_ST_PASS: u8 = 0xAA;
const KBD_RES_ST_FAIL_A: u8 = 0xFC;
const KBD_RES_ST_FAIL_B: u8 = 0xFD;

//
// Keyboard command constants
//
// Terminology:
// TM = Typematic
// AR = Autorepeat
// MK = Make
// RE = Release
//

const KBD_COM_LED: u8 = 0xED;
const KBD_COM_ECHO: u8 = 0xEE;
const KBD_COM_SCANCODE: u8 = 0xF0;
const KBD_COM_IDENTIFY: u8 = 0xF2;
const KBD_COM_TYPEMATIC: u8 = 0xF3;
const KBD_COM_SCAN_ON: u8 = 0xF4;
const KBD_COM_SCAN_OFF: u8 = 0xF5;
const KBD_COM_SET_DEFAULT: u8 = 0xF6;
const KBD_COM_TM_AR_ALL: u8 = 0xF7;
const KBD_COM_MK_RE_ALL: u8 = 0xF8;
const KBD_COM_MK_ALL: u8 = 0xF9;
const KBD_COM_TM_AR_MK_RE_ALL: u8 = 0xFA;
const KBD_COM_TM_AR_SINGLE: u8 = 0xFB;
const KBD_COM_MK_RE_SINGLE: u8 = 0xFC;
const KBD_COM_MK_SINGLE: u8 = 0xFD;
const KBD_COM_RESEND: u8 = 0xFE;
const KBD_COM_SELF_TEST: u8 = 0xFF;

/// Generic PS/2 keyboard
pub struct PS2Keyboard;
impl PS2Keyboard {
    /// Initialize the PS/2 keyboard
    pub fn init() {
        unsafe {
            // Wait till the KBC is ready
            KBC::wait_ready();

            // Run self test
            PS2Keyboard::run_self_test();

            // Reset LEDs
            PS2Keyboard::set_leds(0x00);

            // Set scancode-set 2
            PS2Keyboard::set_scan_table(0x02);
            if let Some(code) = PS2Keyboard::get_scan_table() {
                println!("[ps2kbd] info: verified usage of scan table {}", code);
            }

            // Enable
            KBC::write_byte(KBD_COM_SCAN_ON);
            KBC::wait_ready();
        }

        // Mark the keyboard as initialized
        *KEYBOARD_INITIALIZED.lock() = true;
    }

    unsafe fn run_self_test() {
        PS2Keyboard::_run_self_test(false);
    }

    unsafe fn _run_self_test(resent: bool) {
        KBC::write_byte(KBD_COM_SELF_TEST);
        match KBC::read_byte() {
            KBD_RES_ST_PASS => println!("[ps2kbd] info: self test passed"),
            KBD_RES_ST_FAIL_A | KBD_RES_ST_FAIL_B => println!("[ps2kbd] error: self test failed"),
            KBD_RES_RESEND if !resent => PS2Keyboard::_run_self_test(true),
            KBD_RES_RESEND => println!("[ps2kbd] error: unable to run self test"),
            b => println!("[ps2kbd] error: invalid response: {:x}", b),
        }
    }

    unsafe fn set_leds(byte: u8) {
        PS2Keyboard::_set_leds(byte, false);
    }

    unsafe fn _set_leds(byte: u8, resent: bool) {
        KBC::write_byte(KBD_COM_LED);
        KBC::write_byte(byte);
        match KBC::read_byte() {
            KBD_RES_ACK => println!("[ps2kbd] info: updated led status"),
            KBD_RES_RESEND if !resent => PS2Keyboard::_set_leds(byte, true),
            KBD_RES_RESEND => println!("[ps2kbd] error: unable to set led status"),
            b => println!("[ps2kbd] error: invalid response: {:x}", b),
        }
    }

    unsafe fn set_scan_table(code: u8) {
        PS2Keyboard::_set_scan_table(code, false);
    }

    unsafe fn _set_scan_table(code: u8, resent: bool) {
        KBC::write_byte(KBD_COM_SCANCODE);
        KBC::write_byte(code);
        match KBC::read_byte() {
            KBD_RES_ACK => println!("[ps2kbd] info: setting scan table {}", code),
            KBD_RES_RESEND if !resent => PS2Keyboard::_set_scan_table(code, true),
            KBD_RES_RESEND => println!("[ps2kbd] error: unable to set scan table"),
            b => println!("[ps2kbd] error: invalid response: {:x}", b),
        }
    }

    unsafe fn get_scan_table() -> Option<u8> {
        PS2Keyboard::_get_scan_table(false)
    }

    unsafe fn _get_scan_table(resent: bool) -> Option<u8> {
        KBC::write_byte(KBD_COM_SCANCODE);
        match KBC::read_byte() {
            KBD_RES_ACK => {
                KBC::write_byte(0x00);
                match KBC::read_byte() {
                    0x43 => Some(1),
                    0x41 => Some(2),
                    0x3f => Some(3),
                    _ => None,
                }
            }
            KBD_RES_RESEND if !resent => PS2Keyboard::_get_scan_table(true),
            KBD_RES_RESEND => {
                println!("[ps2kbd] error: unable to get scan table");
                None
            }
            resp => {
                println!("[ps2kbd] error: invalid response: 0x{:x}", resp);
                None
            }
        }
    }
}

fn read_next_key() {
    let data = unsafe { KBC::read_byte() };
    let mut kbd = KEYBOARD.lock();
    match kbd.add_byte(data) {
        Ok(Some(event)) => {
            let key = kbd.process_keyevent(event.clone());
            if key.is_some() {
                match key.unwrap() {
                    DecodedKey::RawKey(code) => print!("{:?}", code),
                    DecodedKey::Unicode(chr) => print!("{}", chr),
                }
            } else {
                // println!("[ps2kbd] Key event is none: {:?}; {:?}", event.code, event.state);
            }
        }
        Ok(None) => (),
        Err(_) => (),
    };
}

pub extern "x86-interrupt" fn handle_interrupt(_stack_frame: &mut ExceptionStackFrame) {
    // Is the keyboard already initialized?
    if *KEYBOARD_INITIALIZED.lock() {
        // Process the next key
        read_next_key();
    } else {
        // Discard the key
        unsafe {
            KBC::read_byte();
        }
    }

    // Notify the PIC
    unsafe {
        PIC8259::get_chained_pics()
            .lock()
            .notify_end_of_interrupt(crate::idt::INT_KBD);
    }
}
