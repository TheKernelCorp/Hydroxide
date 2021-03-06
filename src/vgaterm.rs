use alloc::boxed::Box;
use core::ptr;
use core::ptr::NonNull;
use core::ptr::Unique;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::port::Port;

use crate::hal::DEVICE_MANAGER;

/// The address of the framebuffer in memory.
pub const VGA_PTR: usize = 0xB8000;

const VGA_SIZE: usize = VGA_WIDTH * VGA_HEIGHT;
const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;

macro_rules! color {
    ($fc:expr, $bc:expr) => {
        bc << 4 | fc
    };
}

macro_rules! chattr {
    ($b:expr, $c:expr) => {
        u16::from($c) << 8 | u16::from($b)
    };
}

macro_rules! offset {
    ($x:expr, $y:expr) => {
        $y * VGA_WIDTH + $x
    };
}

type TerminalBuffer = Unique<[u16; VGA_SIZE]>;

pub struct TerminalDevice {
    x: usize,
    y: usize,
    color: u8,
    buf: TerminalBuffer,
}

impl TerminalDevice {
    pub fn init(name: &'static str, ptr: usize) -> Result<(), &str> {
        let mut term = TerminalDevice {
            x: 0,
            y: 0,
            color: 0x07,
            buf: Unique::new(ptr as *mut _).unwrap(),
        };
        term.clear();
        DEVICE_MANAGER
            .lock()
            .register_device(name, box term)
            .unwrap();
        Ok(())
    }

    pub fn clear(&mut self) {
        let chr = chattr!(b' ', self.color);
        let buf = unsafe { self.buf.as_mut() };
        #[allow(clippy::needless_range_loop)]
        for i in 0..VGA_SIZE {
            buf[i] = chr;
        }
        self.x = 0;
        self.y = 0;
    }

    fn write_u8(&mut self, byte: u8) {
        match byte {
            // Carriage return
            b'\r' => self.x = 0,

            // Line feed
            b'\n' => self.new_line(),

            // Tab
            b'\t' => {
                const TAB_SIZE: usize = 2;
                for _ in 0..(TAB_SIZE - (self.x % TAB_SIZE)) {
                    self.write_u8(b' ');
                }
            }

            // Backspace
            0x08 => {
                let chr = chattr!(b' ', self.color);
                let buf = unsafe { self.buf.as_mut() };
                match self.x {
                    0 if self.y > 0 => {
                        self.y -= 1;
                        self.x = VGA_WIDTH - 1;
                    }
                    _ if self.x > 0 => self.x -= 1,
                    _ => (),
                }
                let off = offset!(self.x, self.y);
                buf[off] = chr;
            }

            // Anything else
            _ => {
                if self.x >= VGA_WIDTH {
                    self.new_line();
                }
                let chr = chattr!(byte, self.color);
                let off = offset!(self.x, self.y);
                self.x += 1;
                unsafe {
                    self.buf.as_mut()[off] = chr;
                }
            }
        }
    }

    fn new_line(&mut self) {
        self.x = 0;
        if self.y < VGA_HEIGHT - 1 {
            self.y += 1;
        } else {
            self.scroll();
        }
    }

    fn scroll(&mut self) {
        let buf = unsafe { self.buf.as_mut() };
        for y in 1..VGA_HEIGHT {
            for x in 0..VGA_WIDTH {
                let off_cur = offset!(x, y);
                let off_new = offset!(x, y - 1);
                buf[off_new] = buf[off_cur];
            }
        }
        let chr_filler = chattr!(b' ', self.color);
        for x in 0..VGA_WIDTH {
            buf[VGA_SIZE - VGA_WIDTH + x] = chr_filler;
        }
    }

    fn update_physical_cursor(&mut self) {
        let off = offset!(self.x, self.y);
        let mut addr = Port::new(0x03D4);
        let mut data = Port::new(0x03D5);
        unsafe {
            addr.write(0x0E_u8);
            data.write((off >> 0x08) as u8);
            addr.write(0x0F_u8);
            data.write((off & 0xFF) as u8);
        }
    }
}

use crate::hal::{Device, DeviceType};
use core::any::Any;

impl Device for TerminalDevice {
    fn get_type(&self) -> DeviceType {
        return DeviceType::CharDevice;
    }

    fn write_byte(&mut self, at: usize, val: u8) {
        self.write_u8(val);
    }

    fn write_bytes(&mut self, at: usize, val: &[u8], len: usize) {
        for b in val {
            self.write_byte(0, *b);
        }
        self.update_physical_cursor();
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

unsafe impl Send for TerminalDevice {}

unsafe impl Sync for TerminalDevice {}

impl core::fmt::Write for TerminalDevice {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let len = s.len();
        self.write_bytes(0, bytes, len);
        Ok(())
    }
}
