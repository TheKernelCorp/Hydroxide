#![allow(dead_code)]

use x86_64::structures::idt::ExceptionStackFrame;
use x86_64::instructions::port::Port;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::pic::PIC8259;

// TODO: Rethink that design and actually use it

enum KeyAction {
    Press(Key),
    Release(Key),
}

enum Key {
    Char(char),
    Modifier {
        ctrl: bool,
        lalt: bool,
        ralt: bool,
        lshift: bool,
        rshift: bool,
    },
}

//
// Key maps
//

static KEYMAP_EN_US: &'static str = "\0\x1b1234567890-=\x08\tqwertyuiop[]\x0a\0asdfghjkl;'`\0\x5czxcvbnm,./\0\0 \0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0-\0\0\0+\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0!@#$%^&*()_+\0\0QWERTYUIOP{}\x0a\0ASDFGHJKL:\x22~\0|ZXCVBNM<>?\0\0 \0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0-\0\0\0+\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";

//
// Keyboard registers
//

const KBD_DATA : u16 = 0x60;
const KBD_STATUS: u16 = 0x64;

//
// Keyboard I/O ports
//
lazy_static! {

    /// Data port
    pub static ref KBD_DATA_PORT: Mutex<Port<u8>> =
        Mutex::new(Port::new(KBD_DATA));

    /// Status port
    pub static ref KBD_STATUS_PORT: Mutex<Port<u8>> =
        Mutex::new(Port::new(KBD_STATUS));
}

//
// Global state
//

// TODO: This should be removed in the future.
// TODO: I'm thinking of serialization of key actions using enums
lazy_static! {
    pub static ref LSHIFT: Mutex<bool> = Mutex::new(false);
    pub static ref RSHIFT: Mutex<bool> = Mutex::new(false);
}

//
// Keyboard responses
//
// Terminology:
// ACK = Acknowledge
// ST = Self test
//

const KBD_RES_ACK: u16 = 0xFA;
const KBD_RES_ECHO: u16 = 0xEE;
const KBD_RES_RESEND: u16 = 0xFE;
const KBD_RES_ERROR_A: u16 = 0x00;
const KBD_RES_ERROR_B: u16 = 0xFF;
const KBD_RES_ST_PASS: u16 = 0xAA;
const KBD_RES_ST_FAIL_A: u16 = 0xFC;
const KBD_RES_ST_FAIL_B: u16 = 0xFD;

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

            // Reset LEDs
            PS2Keyboard::send_byte(KBD_COM_LED);
            PS2Keyboard::send_byte(0x00);

            // Set fastest refresh rate
            PS2Keyboard::send_byte(KBD_COM_TYPEMATIC);
            PS2Keyboard::send_byte(0x00);

            // Enable
            PS2Keyboard::send_byte(KBD_COM_SCAN_ON);
        }
    }

    /// Acknowledge the keyboard status
    unsafe fn ack() {
        while KBD_STATUS_PORT.lock().read() & 0x2 != 0 {}
    }

    /// Send a byte to the keyboard data port
    unsafe fn send_byte(com: u8) {
        PS2Keyboard::ack();
        KBD_DATA_PORT.lock().write(com);
    }
}

fn read_next_key() {
    let mut data = unsafe { KBD_DATA_PORT.lock().read() };
    let pressed = data & 0x80 == 0;
    data &= if pressed { 0xFF } else { !0x80 };

    if pressed {
        if data == 42 {
            *LSHIFT.lock() = true;
            return;
        }
        if data == 55 {
            *RSHIFT.lock() = true;
            return;
        }
        if *LSHIFT.lock() || *RSHIFT.lock() {
            data += 128;
        }
        // TODO: Write to buffer instead of printing
        print!("{}", KEYMAP_EN_US.chars().nth(data as usize).unwrap());
    } else {
        if data == 42 {
            *LSHIFT.lock() = false;
            return;
        }
        if data == 55 {
            *RSHIFT.lock() = false;
            return;
        }
    }
}

pub extern "x86-interrupt" fn handle_interrupt(_stack_frame: &mut ExceptionStackFrame) {
    read_next_key();
    unsafe {
        PIC8259
            ::get_chained_pics()
            .lock()
            .notify_end_of_interrupt(crate::idt::INT_KBD);
    }
}