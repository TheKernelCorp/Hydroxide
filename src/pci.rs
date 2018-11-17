use core::convert::From;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::{instructions::port::Port, structures::paging::PageTableFlags, PhysAddr};

use crate::paging::PAGING;

const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

lazy_static! {
  static ref PCI_CONFIG_ADDRESS: Mutex<Port<u32>> = Mutex::new(Port::new(CONFIG_ADDRESS));
  static ref PCI_CONFIG_DATA: Mutex<Port<u32>> = Mutex::new(Port::new(CONFIG_DATA));
}

const PCIFIELD_VENDOR_ID: u8 = 0x00;
const PCIFIELD_DEVICE_ID: u8 = 0x02;
const PCIFIELD_REVISION_ID: u8 = 0x08;
const PCIFIELD_PROG_IF: u8 = 0x09;
const PCIFIELD_SUBCLASS: u8 = 0x0A;
const PCIFIELD_CLASS: u8 = 0x0B;
const PCIFIELD_HHEADER_TYPE: u8 = 0x0E;
const PCIFIELD_SECONDARY_BUS_NUMBER: u8 = 0x19;

#[derive(Clone, Copy)]
pub struct PCIDeviceAddress {
  bus: u8,
  slot: u8,
  func: u8,
}

impl From<&PCIDeviceAddress> for u32 {
  fn from(addr: &PCIDeviceAddress) -> Self {
    assert!(addr.slot < 1 << 5);
    assert!(addr.func < 1 << 3);

    ((addr.func as u32) << 8u32)
      | ((addr.slot as u32) << 11u32)
      | ((addr.bus as u32) << 16u32)
      | 1u32 << 31u32
  }
}

impl From<PCIDeviceAddress> for u32 {
  fn from(addr: PCIDeviceAddress) -> Self {
    assert!(addr.slot < 1 << 5);
    assert!(addr.func < 1 << 3);

    ((addr.func as u32) << 8u32)
      | ((addr.slot as u32) << 11u32)
      | ((addr.bus as u32) << 16u32)
      | 1u32 << 31u32
  }
}

#[derive(Clone, Copy)]
pub struct PCIDeviceID {
  pub vendor_id: u16,
  pub device_id: u16,
}

impl PCIDeviceID {
  fn is_valid(&self) -> bool {
    if self.vendor_id == 0xFFFF && self.device_id == 0xFFFF {
      return false;
    }
    true
  }
}

pub struct PCIFind {
  vendor_id: u16,
  device_id: u16,
  class_id: u8,
  subclass_id: u8,
  prog_if: u8,
  rev_id: u8,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct PCIDeviceType {
  class_id: u8,
  subclass_id: u8,
  prog_if: u8,
  rev_id: u8,
}

#[repr(C)]
union PCIData {
  val8: [u8; 4],
  val16: [u16; 2],
  val32: u32,
}

impl PCIFind {
  pub fn new(vendor_id: u16, device_id: u16) -> Self {
    PCIFind {
      vendor_id,
      device_id,
      class_id: 0xFFu8,
      subclass_id: 0xFFu8,
      prog_if: 0xFFu8,
      rev_id: 0xFFu8,
    }
  }

  fn matches(&self, id: &PCIDeviceID, dev_type: &PCIDeviceType) -> bool {
    if id.vendor_id == 0xFFFF && id.device_id == 0xFFFF {
      return false;
    }
    if self.vendor_id != 0xFFFF && id.vendor_id != self.vendor_id {
      return false;
    }
    if self.device_id != 0xFFFF && id.device_id != self.device_id {
      return false;
    }
    if self.class_id != 0xFF && dev_type.class_id != self.class_id {
      return false;
    }
    if self.subclass_id != 0xFF && dev_type.subclass_id != self.subclass_id {
      return false;
    }
    if self.prog_if != 0xFF && dev_type.prog_if != self.prog_if {
      return false;
    }
    if self.rev_id != 0xFF && dev_type.rev_id != self.rev_id {
      return false;
    }
    true
  }
}

#[derive(Clone, Copy)]
pub struct PCIDevice {
  pub address: PCIDeviceAddress,
  id: PCIDeviceID,
  dev_type: PCIDeviceType,
}

impl PCIDevice {
  pub unsafe fn pci_read32(address: &PCIDeviceAddress, offset: u8) -> u32 {
    assert!(offset & 0x3 == 0);
    PCI_CONFIG_ADDRESS
      .lock()
      .write(u32::from(address) + offset as u32);
    PCI_CONFIG_DATA.lock().read()
  }

  pub unsafe fn pci_write32(address: &PCIDeviceAddress, offset: u8, val: u32) {
    assert!(offset & 0x3 == 0);
    PCI_CONFIG_ADDRESS
      .lock()
      .write(u32::from(address) + offset as u32);
    PCI_CONFIG_DATA.lock().write(val);
  }

  pub unsafe fn pci_read16(address: &PCIDeviceAddress, offset: u8) -> u16 {
    assert!(offset & 0x1 == 0);
    let aligned_offset = offset & !0x3;
    let data_raw = PCIDevice::pci_read32(address, aligned_offset);
    let data: PCIData = PCIData { val32: data_raw };
    data.val8[(offset & 0x3) as usize] as u16
      | ((data.val8[(offset & 0x3) as usize + 1] as u16) << 8)
  }

  pub unsafe fn pci_read8(address: &PCIDeviceAddress, offset: u8) -> u8 {
    let aligned_offset = offset & !0x3;
    let data_raw = PCIDevice::pci_read32(address, aligned_offset);
    let data: PCIData = PCIData { val32: data_raw };
    data.val8[(offset & 0x3) as usize]
  }

  pub fn read8(&self, offset: u8) -> u8 {
    unsafe { PCIDevice::pci_read8(&self.address, offset) }
  }

  pub fn read16(&self, offset: u8) -> u16 {
    unsafe { PCIDevice::pci_read16(&self.address, offset) }
  }

  pub fn read32(&self, offset: u8) -> u32 {
    unsafe { PCIDevice::pci_read32(&self.address, offset) }
  }

  pub fn write32(&self, offset: u8, val: u32) {
    unsafe { PCIDevice::pci_write32(&self.address, offset, val) }
  }

  fn get_id(address: &PCIDeviceAddress) -> PCIDeviceID {
    PCIDeviceID {
      device_id: unsafe { PCIDevice::pci_read16(address, PCIFIELD_DEVICE_ID) },
      vendor_id: unsafe { PCIDevice::pci_read16(address, PCIFIELD_VENDOR_ID) },
    }
  }

  fn get_type(address: &PCIDeviceAddress) -> PCIDeviceType {
    PCIDeviceType {
      class_id: unsafe { PCIDevice::pci_read8(address, PCIFIELD_CLASS) },
      subclass_id: unsafe { PCIDevice::pci_read8(address, PCIFIELD_SUBCLASS) },
      prog_if: unsafe { PCIDevice::pci_read8(address, PCIFIELD_PROG_IF) },
      rev_id: unsafe { PCIDevice::pci_read8(address, PCIFIELD_REVISION_ID) },
    }
  }

  fn matches_pattern(address: &PCIDeviceAddress, pattern: &PCIFind) -> Option<PCIDevice> {
    let id = PCIDevice::get_id(&address.clone());
    if !id.is_valid() {
      return None;
    }
    let dev_type = PCIDevice::get_type(&address.clone());
    if pattern.matches(&id, &dev_type) {
      return Some(PCIDevice {
        address: address.clone(),
        id,
        dev_type,
      });
    }
    None
  }

  fn find_on_bus(bus: u8, find: &PCIFind, last: Option<u32>) -> Option<PCIDevice> {
    let mut next_device = 0u32;
    let mut found_device = None;

    for slot in 0..32 {
      let mut num_func = 1u32;
      let mut func = 0;
      while func < num_func {
        let ref addr = PCIDeviceAddress {
          bus,
          slot,
          func: func as u8,
        };
        let devaddr: u32 = addr.clone().into();
        if last.unwrap_or(0u32) < devaddr && (found_device.is_none() || devaddr < next_device) {
          match PCIDevice::matches_pattern(addr, find) {
            Some(dev) => {
              found_device = Some(dev);
              next_device = devaddr;
            }
            _ => {}
          }
        }
        let header = unsafe { PCIDevice::pci_read8(&addr, PCIFIELD_HHEADER_TYPE) };
        if header & 0x80 == 0x80 {
          num_func = 8;
        }
        if (header & 0x7f) == 0x1 {
          let sub_bus_id = unsafe { PCIDevice::pci_read8(&addr, PCIFIELD_SECONDARY_BUS_NUMBER) };
          let rec_ret = PCIDevice::find_on_bus(sub_bus_id, find, last);
          let ref addr = rec_ret.unwrap().address;
          if last.unwrap_or(0) < addr.into()
            && (found_device.is_none() || u32::from(addr) < next_device)
          {
            found_device = rec_ret;
          }
        }
        func += 1;
      }
    }

    return found_device;
  }

  pub fn search(find: &PCIFind, last: Option<u32>) -> Option<PCIDevice> {
    PCIDevice::find_on_bus(0, find, last)
  }

  pub fn get_bar(&self, bar: u8) -> PCIBAR {
    let lo = self.read32(0x10 + 4 * (bar + 0));

    let mut res = PCIBAR {
      addr_raw: lo as u64,
      size_raw: 0,
    };

    if res.is_64bit() {
      let hi = self.read32(0x10 + 4 * (bar + 1));
      res.addr_raw |= (hi as u64) << 32;
      self.write32(0x10 + 4 * (bar + 0), 0xFFFFFFFF);
      self.write32(0x10 + 4 * (bar + 1), 0xFFFFFFFF);
      let size_lo = self.read32(0x10 + 4 * (bar + 0));
      let size_hi = self.read32(0x10 + 4 * (bar + 1));
      self.write32(0x10 + 4 * (bar + 0), lo);
      self.write32(0x10 + 4 * (bar + 1), hi);
      res.size_raw = (size_hi as u64) << 32 | ((size_lo as u64) << 0);
      res.size_raw = !(res.size_raw & 0xFFFFFFFFFFFFFFF0) + 1;
    } else if res.is_32bit() {
      self.write32(0x10 + 4 * (bar + 0), 0xFFFFFFFF);
      let size_lo = self.read32(0x10 + 4 * (bar + 0));
      self.write32(0x10 + 4 * (bar + 0), lo);
      res.size_raw = (size_lo as u64) << 0;
      res.size_raw = !(res.size_raw & 0xFFFFFFF0) + 1;
      res.size_raw &= 0xFFFFFFFF;
    } else if res.is_iospace() {
      self.write32(0x10 + 4 * (bar + 0), 0xFFFFFFFF);
      let size_lo = self.read32(0x10 + 4 * (bar + 0));
      self.write32(0x10 + 4 * (bar + 0), lo);
      res.size_raw = (size_lo as u64) << 0;
      res.size_raw = !(res.size_raw & 0xFFFFFFFC) + 1;
      res.size_raw &= 0xFFFFFFFF;
    }

    res
  }
}

const PCIBAR_TYPE_IOSPACE: u8 = 0x0 << 1 | 0x1 << 0;
const PCIBAR_TYPE_16BIT: u8 = 0x1 << 1 | 0x0 << 0;
const PCIBAR_TYPE_32BIT: u8 = 0x0 << 1 | 0x0 << 0;
const PCIBAR_TYPE_64BIT: u8 = 0x2 << 1 | 0x0 << 0;

pub struct PCIBAR {
  addr_raw: u64,
  size_raw: u64,
}

impl PCIBAR {
  pub fn addr(&self) -> u64 {
    match self.is_iospace() {
      true => self.addr_raw & 0xFFFFFFFFFFFFFFFC,
      false => self.addr_raw & 0xFFFFFFFFFFFFFFF0,
    }
  }

  pub fn size(&self) -> u64 {
    self.size_raw & 0xFFFFFFFFFFFFFFFF
  }

  pub fn get_type(&self) -> u8 {
    let raw = self.addr_raw as u8;
    match raw & 0x3 {
      PCIBAR_TYPE_IOSPACE => raw & 0x3,
      _ => raw & 0x7,
    }
  }

  pub fn is_iospace(&self) -> bool {
    self.get_type() == PCIBAR_TYPE_IOSPACE
  }

  pub fn is_16bit(&self) -> bool {
    self.get_type() == PCIBAR_TYPE_16BIT
  }

  pub fn is_32bit(&self) -> bool {
    self.get_type() == PCIBAR_TYPE_32BIT
  }

  pub fn is_64bit(&self) -> bool {
    self.get_type() == PCIBAR_TYPE_64BIT
  }

  pub fn is_mmio(&self) -> bool {
    self.is_16bit() || self.is_32bit() || self.is_64bit()
  }

  pub fn identity_map(&self) -> Result<(), &'static str> {
    if !self.is_mmio() {
      return Err("BAR is not mmio");
    }

    let addr = self.addr();
    let size = self.size();

    PAGING.lock().identity_map(
      PhysAddr::new(addr),
      PhysAddr::new(addr + size),
      PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
      false,
    );

    Ok(())
  }
}

/*pub fn tmp_init_devs() {
  let bge_dev = PCIDevice::search(&PCIFind::new(0x1234, 0x1111), None);

  match bge_dev {
    Some(dev) => {
      const VBE_DISPI_GETCAPS: u16 = 2;
      const VBE_DISPI_NUM_REGISTERS: u16 = 10;

      const VBE_DISPI_INDEX_ID: u16 = 0;
      const VBE_DISPI_INDEX_XRES: u16 = 1;
      const VBE_DISPI_INDEX_YRES: u16 = 2;
      const VBE_DISPI_INDEX_BPP: u16 = 3;
      const VBE_DISPI_INDEX_ENABLE: u16 = 4;

      fn read_reg(mmio: u64, index: u16) -> u16 {
        use core::ptr::Unique;
        assert!(index < VBE_DISPI_NUM_REGISTERS);
        let regs: Unique<[u16; VBE_DISPI_NUM_REGISTERS as usize]> =
          Unique::new((mmio + 0x500) as *mut _).unwrap();
        unsafe { regs.as_ref()[index as usize] }
      }

      fn write_reg(mmio: u64, index: u16, val: u16) {
        use core::ptr::Unique;
        assert!(index < VBE_DISPI_NUM_REGISTERS);
        let mut regs: Unique<[u16; VBE_DISPI_NUM_REGISTERS as usize]> =
          Unique::new((mmio + 0x500) as *mut _).unwrap();
        unsafe { regs.as_mut()[index as usize] = val };
      }

      fn get_capability(mmio: u64, index: u16) -> u16 {
        let was_enabled = read_reg(mmio, VBE_DISPI_INDEX_ENABLE);
        print!("{}", ""); // FIXME: Investigate issue. It returns 0 without this print with the format! needing to be used
        write_reg(
          mmio,
          VBE_DISPI_INDEX_ENABLE,
          was_enabled | VBE_DISPI_GETCAPS,
        );
        let cap = read_reg(mmio, index);
        write_reg(mmio, VBE_DISPI_INDEX_ENABLE, was_enabled);
        return cap;
      }

      println!("[BGE Adapter @ 0x{:08x}] Found", u32::from(dev.address));
      let fb_bar = dev.get_bar(0);
      println!(
        "[BGE Adapter @ 0x{:08x}] Framebuffer BAR: 0x{:08x}",
        u32::from(dev.address),
        fb_bar.addr()
      );
      let mmio_bar = dev.get_bar(2);
      mmio_bar.identity_map();
      if mmio_bar.is_mmio() {
        println!(
          "[BGE Adapter @ 0x{:08x}] MMIO BAR: 0x{:08x}",
          u32::from(dev.address),
          mmio_bar.addr()
        );
      }
      println!(
        "[BGE Adapter @ 0x{:08x}] Version: 0x{:04x}",
        u32::from(dev.address),
        read_reg(mmio_bar.addr(), VBE_DISPI_INDEX_ID)
      );
      let bpp = get_capability(mmio_bar.addr(), VBE_DISPI_INDEX_BPP);
      let width = get_capability(mmio_bar.addr(), VBE_DISPI_INDEX_XRES);
      let height = get_capability(mmio_bar.addr(), VBE_DISPI_INDEX_YRES);
      println!(
        "[BGE Adapter @ 0x{:08x}] Max BPP: {}",
        u32::from(dev.address),
        bpp
      );
      println!(
        "[BGE Adapter @ 0x{:08x}] Max Width: {}",
        u32::from(dev.address),
        width
      );
      println!(
        "[BGE Adapter @ 0x{:08x}] Max Height: {}",
        u32::from(dev.address),
        height
      );
    }
    None => println!("[BGE Adapter] Not found"),
  };
}*/
