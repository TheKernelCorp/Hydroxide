use core::ptr::Unique;

use lazy_static::lazy_static;

use crate::pci::{PCIDevice, PCIFind, PCIBAR};

const VBE_DISPI_GETCAPS: u16 = 2;
const VBE_DISPI_NUM_REGISTERS: u16 = 10;

const VBE_DISPI_INDEX_ID: u16 = 0;
const VBE_DISPI_INDEX_XRES: u16 = 1;
const VBE_DISPI_INDEX_YRES: u16 = 2;
const VBE_DISPI_INDEX_BPP: u16 = 3;
const VBE_DISPI_INDEX_ENABLE: u16 = 4;

lazy_static! {
  static ref BGA_SIGNATURE: PCIFind = PCIFind::new(0x1234, 0x1111);
}

pub struct BochsGraphicsAdapter {
  pci_device: PCIDevice,
  pub max_bpp: u16,
  pub max_width: u16,
  pub max_height: u16,
  framebuffer_bar: PCIBAR,
  mmio_bar: PCIBAR,
  registers: Unique<[u16; VBE_DISPI_NUM_REGISTERS as usize]>,
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
      max_bpp: 0u16,
      max_width: 0u16,
      max_height: 0u16,
      framebuffer_bar: fb_bar,
      mmio_bar: mmio_bar,
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
    self.max_width = max_width;
    self.max_height = max_height;

    self
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
