use core::ptr::Unique;

use lazy_static::lazy_static;

use alloc::prelude::*;
use alloc::slice;
use alloc::boxed::*;

use rlibc::memcpy;

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

pub struct VideoDevice<'a, T> where T: GraphicsProvider {
    pub provider: &'a T,
    pub mode: VideoMode,
    pub buffer: Vec<u32>,
}

impl<'a, T> VideoDevice<'a, T> where T: GraphicsProvider {
    pub fn new(provider: &'a T, mode: &VideoMode) -> VideoDevice<'a, T> {
        Self {
            provider,
            mode: mode.clone(),
            buffer: vec![0u32; mode.width * mode.height],
        }
    }

    pub fn flush(self) {
        let len = self.buffer.len();
        let data = self.buffer.into_boxed_slice();
        unsafe {
            memcpy(Box::into_raw(self.provider.get_framebuffer(&self.mode)) as *mut _, Box::into_raw(data) as *const _, len);
        }
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

        fb_bar.identity_map();
        mmio_bar.identity_map();

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
            None => Err("Could not find Bochs Graphics Adaptetr"),
        }
    }
}
