use alloc::boxed::*;
use alloc::prelude::*;
use alloc::slice;
use alloc::vec;
use core::ptr::Unique;
use lazy_static::lazy_static;
use rlibc::memcpy;

use crate::ansi::{Ansi, AnsiEscape};

use crate::pci::{PCIDevice, PCIFind, PCIBAR};

const VBE_DISPI_GETCAPS: u16 = 2;
const VBE_DISPI_NUM_REGISTERS: u16 = 10;

const VBE_DISPI_INDEX_ID: u16 = 0;
const VBE_DISPI_INDEX_XRES: u16 = 1;
const VBE_DISPI_INDEX_YRES: u16 = 2;
const VBE_DISPI_INDEX_BPP: u16 = 3;
const VBE_DISPI_INDEX_ENABLE: u16 = 4;

const VBE_DISPI_DISABLED: u16 = 0;
const VBE_DISPI_ENABLED: u16 = 1;

const VBE_DISPI_LFB_ENABLED: u16 = 64;
const VBE_DISPI_NOCLEAR: u16 = 128;

lazy_static! {
    static ref BGA_SIGNATURE: PCIFind = PCIFind::new(0x1234, 0x1111);
    static ref DEFAULT_VIDEO_MODE: VideoMode = VideoMode {
        width: 1280,
        height: 720,
        bpp: 32
    };
}

pub static FONT: &'static [u8] = include_bytes!("unifont.font");

pub trait TerminalProvider {
    fn get_width(&self) -> usize;
    fn get_height(&self) -> usize;

    fn get_char_width(&self) -> usize;
    fn get_char_height(&self) -> usize;

    fn draw_char(&mut self, x: usize, y: usize, character: char, fg: u32, bg: u32);
}

pub struct TerminalDriver<'a> {
    x: usize,
    y: usize,
    fg_def: u32,
    fg: u32,
    bg_def: u32,
    bg: u32,
    provider: &'a mut TerminalProvider,
}

impl<'a> TerminalDriver<'a> {
    pub fn new(provider: &'a mut TerminalProvider) -> TerminalDriver<'a> {
        TerminalDriver {
            x: 0,
            y: 0,
            fg_def: Ansi::color(7),
            fg: Ansi::color(7),
            bg_def: Ansi::color(0),
            bg: Ansi::color(0),
            provider,
        }
    }

    pub fn write_str(&mut self, s: &str) {
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        'outer: while i < chars.len() {
            match chars[i] {
                '\x1b' => {
                    i += 1;
                    if i >= chars.len() || chars[i] != '[' {
                        break 'outer;
                    }
                    i += 1;
                    let (codes, skip) = Ansi::parse(&chars[i..]);
                    for code in codes {
                        match code {
                            None => {}
                            Some(AnsiEscape::Reset) => {
                                self.reset();
                            }
                            Some(AnsiEscape::Foreground(color)) => {
                                self.fg = Ansi::color(color);
                            }
                            Some(AnsiEscape::Background(color)) => {
                                self.bg = Ansi::color(color);
                            }
                        }
                    }
                    i += skip;
                }
                _ => self.write_car(chars[i]),
            }

            i += 1;
        }

        self.reset();
    }

    pub fn write_car(&mut self, c: char) {
        match c {
            '\n' => self.new_line(),
            _ => {
                if self.x >= self.provider.get_width() {
                    self.new_line();
                }
                self.provider.draw_char(
                    self.x * self.provider.get_char_width(),
                    self.y * self.provider.get_char_height(),
                    c,
                    self.fg,
                    self.bg,
                );
                self.x += 1;
            }
        }
    }

    pub fn reset(&mut self) {
        self.fg = self.fg_def;
        self.bg = self.bg_def;
    }

    pub fn set_fg(&mut self, color: u32) {
        self.fg_def = color;
    }

    pub fn set_bg(&mut self, color: u32) {
        self.fg_def = color;
    }

    pub fn new_line(&mut self) {
        self.x = 0;
        if self.y < self.provider.get_height() - 1 {
            self.y += 1;
        } else {
            // TODO: Scroll
        }
    }
}

impl<'a> core::fmt::Write for TerminalDriver<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

impl<'a, T> TerminalProvider for VideoDevice<'a, T>
where
    T: GraphicsProvider,
{
    fn get_width(&self) -> usize {
        self.mode.width / 8
    }

    fn get_height(&self) -> usize {
        self.mode.height / 16
    }

    fn get_char_width(&self) -> usize {
        8
    }

    fn get_char_height(&self) -> usize {
        16
    }

    fn draw_char(&mut self, x: usize, y: usize, character: char, fg: u32, bg: u32) {
        if x + 8 <= self.mode.width && y + 16 <= self.mode.height {
            let font_i = 16 * (character as usize);
            let mut dst = self.buffer.as_mut_ptr() as usize + (x + y * self.mode.width) * 4;

            if font_i + 16 <= FONT.len() {
                for row in 0..16 {
                    let row_data = FONT[font_i + row];
                    for col in 0..8 {
                        if row_data >> (7 - col) & 1 == 1 {
                            unsafe {
                                *((dst + col * 4) as *mut u32) = fg;
                            }
                        } else {
                            unsafe {
                                *((dst + col * 4) as *mut u32) = bg;
                            }
                        }
                    }
                    dst += self.mode.width * 4;
                }
            }
        }
    }
}

pub struct VideoDevice<'a, T>
where
    T: GraphicsProvider,
{
    pub provider: &'a T,
    pub mode: VideoMode,
    pub buffer: Vec<u32>,
}

impl<'a, T> VideoDevice<'a, T>
where
    T: GraphicsProvider,
{
    pub fn new(provider: &'a T, mode: &VideoMode) -> VideoDevice<'a, T> {
        Self {
            provider,
            mode: mode.clone(),
            buffer: vec![0u32; mode.width * mode.height],
        }
    }

    pub fn flush(&self) {
        let fb: Box<&mut [u32]> = self.provider.get_framebuffer(&self.mode);
        fb.copy_from_slice(&self.buffer);
    }
}

pub trait GraphicsProvider {
    fn get_framebuffer(&self, mode: &VideoMode) -> Box<&mut [u32]>;
}

#[derive(Clone)]
pub struct VideoMode {
    pub width: usize,
    pub height: usize,
    pub bpp: u16,
}

pub struct BochsGraphicsAdapter {
    pci_device: PCIDevice,
    pub max_bpp: u16,
    pub max_width: usize,
    pub max_height: usize,
    framebuffer_bar: PCIBAR,
    mmio_bar: PCIBAR,
    registers: Unique<[u16; VBE_DISPI_NUM_REGISTERS as usize]>,
}

impl GraphicsProvider for BochsGraphicsAdapter {
    fn get_framebuffer(&self, mode: &VideoMode) -> Box<&mut [u32]> {
        let size: usize = (mode.width * mode.height) as usize;
        unsafe {
            let slice = slice::from_raw_parts_mut(self.framebuffer_bar.addr() as *mut u32, size);
            box slice
        }
    }
}

impl BochsGraphicsAdapter {
    pub fn new(dev: &PCIDevice) -> Self {
        let fb_bar = dev.get_bar(0);
        let mmio_bar = dev.get_bar(2);
        let mmio = mmio_bar.addr();

        fb_bar
            .identity_map()
            .expect("Unable to map BGA framebuffer!");
        mmio_bar.identity_map().expect("Unable to map BGA mmio!");

        BochsGraphicsAdapter {
            pci_device: *dev,
            max_bpp: 0,
            max_width: 0,
            max_height: 0,
            framebuffer_bar: fb_bar,
            mmio_bar,
            registers: Unique::new((mmio + 0x500) as *mut _).unwrap(),
        }
    }

    pub fn addr(&self) -> u32 {
        u32::from(self.pci_device.address)
    }

    pub fn version(&self) -> u16 {
        self.read_reg(VBE_DISPI_INDEX_ID)
    }

    pub fn init(mut self) -> Self {
        let max_bpp = self.get_capability(VBE_DISPI_INDEX_BPP);
        let max_width = self.get_capability(VBE_DISPI_INDEX_XRES);
        let max_height = self.get_capability(VBE_DISPI_INDEX_YRES);

        self.max_bpp = max_bpp;
        self.max_width = max_width as usize;
        self.max_height = max_height as usize;

        self
    }

    pub fn set_video_mode(&mut self, mode: &VideoMode, clear: bool) {
        let mut enable = VBE_DISPI_ENABLED | VBE_DISPI_LFB_ENABLED;
        if !clear {
            enable |= VBE_DISPI_NOCLEAR;
        }

        self.write_reg(VBE_DISPI_INDEX_ENABLE, VBE_DISPI_DISABLED);
        self.write_reg(VBE_DISPI_INDEX_XRES, mode.width as u16);
        self.write_reg(VBE_DISPI_INDEX_YRES, mode.height as u16);
        self.write_reg(VBE_DISPI_INDEX_BPP, mode.bpp);
        self.write_reg(VBE_DISPI_INDEX_ENABLE, enable);
    }

    pub fn get_default_mode(&self) -> Option<VideoMode> {
        if self.supports_resolution(DEFAULT_VIDEO_MODE.clone()) {
            return Some(DEFAULT_VIDEO_MODE.clone());
        }
        None
    }

    pub fn supports_resolution(&self, mode: VideoMode) -> bool {
        if mode.width > self.max_width || mode.height > self.max_height || mode.bpp > self.max_bpp {
            return false;
        }
        true
    }

    fn read_reg(&self, index: u16) -> u16 {
        assert!(index < VBE_DISPI_NUM_REGISTERS);
        unsafe { self.registers.as_ref()[index as usize] }
    }

    fn write_reg(&mut self, index: u16, val: u16) {
        assert!(index < VBE_DISPI_NUM_REGISTERS);
        unsafe { self.registers.as_mut()[index as usize] = val };
    }

    fn get_capability(&mut self, index: u16) -> u16 {
        let was_enabled = self.read_reg(VBE_DISPI_INDEX_ENABLE);
        self.write_reg(VBE_DISPI_INDEX_ENABLE, was_enabled | VBE_DISPI_GETCAPS);
        let cap = self.read_reg(index);
        assert!(cap != 0); // Someone, if you can find why this is needed, please tell me. I'm desperate
        self.write_reg(VBE_DISPI_INDEX_ENABLE, was_enabled);
        cap
    }

    pub fn detect() -> Result<PCIDevice, &'static str> {
        match PCIDevice::search(&BGA_SIGNATURE, None) {
            Some(dev) => Ok(dev),
            None => Err("Could not find Bochs Graphics Adapter"),
        }
    }
}
