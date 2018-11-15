use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::port::Port;

#[derive(Clone, Copy)]
pub struct PCIDeviceID {
  pub vendor_id: u16,
  pub device_id: u16,
}

#[allow(dead_code)]
struct PCIDeviceType {
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

pub fn make_dev_addr(bus: u8, slot: u8, func: u8) -> u32 {
  assert!(slot < 1 << 5);
  assert!(func < 1 << 3);

  ((func as u32) << 8u32) | ((slot as u32) << 11u32) | ((bus as u32) << 16u32) | 1u32 << 31u32
}

const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

lazy_static! {
  static ref PCI_CONFIG_ADDRESS: Mutex<Port<u32>> = Mutex::new(Port::new(CONFIG_ADDRESS));
  static ref PCI_CONFIG_DATA: Mutex<Port<u32>> = Mutex::new(Port::new(CONFIG_DATA));
}

unsafe fn pci_read_raw32(devaddr: u32, off: u8) -> u32 {
  assert!(off & 0x3 == 0);
  PCI_CONFIG_ADDRESS.lock().write(devaddr + off as u32);
  PCI_CONFIG_DATA.lock().read()
}

unsafe fn pci_read16(devaddr: u32, off: u8) -> u16 {
  assert!(off & 0x1 == 0);
  let aligned_offset = off & !0x3;
  let data_raw = pci_read_raw32(devaddr, aligned_offset);
  let data: PCIData = PCIData { val32: data_raw };
  return data.val8[(off & 0x3) as usize] as u16
    | ((data.val8[(off & 0x3) as usize + 1] as u16) << 8);
}

unsafe fn pci_read8(devaddr: u32, off: u8) -> u8 {
  let aligned_offset = off & !0x3;
  let data_raw = pci_read_raw32(devaddr, aligned_offset);
  let data: PCIData = PCIData { val32: data_raw };
  return data.val8[(off & 0x3) as usize];
}

const PCIFIELD_VENDOR_ID: u8 = 0x00;
const PCIFIELD_DEVICE_ID: u8 = 0x02;
const PCIFIELD_REVISION_ID: u8 = 0x08;
const PCIFIELD_PROG_IF: u8 = 0x09;
const PCIFIELD_SUBCLASS: u8 = 0x0A;
const PCIFIELD_CLASS: u8 = 0x0B;
const PCIFIELD_HHEADER_TYPE: u8 = 0x0E;
const PCIFIELD_SECONDARY_BUS_NUMBER: u8 = 0x19;

fn get_device_id(devaddr: u32) -> PCIDeviceID {
  PCIDeviceID {
    device_id: unsafe { pci_read16(devaddr, PCIFIELD_DEVICE_ID) },
    vendor_id: unsafe { pci_read16(devaddr, PCIFIELD_VENDOR_ID) },
  }
}

fn get_device_type(devaddr: u32) -> PCIDeviceType {
  PCIDeviceType {
    class_id: unsafe { pci_read8(devaddr, PCIFIELD_CLASS) },
    subclass_id: unsafe { pci_read8(devaddr, PCIFIELD_SUBCLASS) },
    prog_if: unsafe { pci_read8(devaddr, PCIFIELD_PROG_IF) },
    rev_id: unsafe { pci_read8(devaddr, PCIFIELD_REVISION_ID) },
  }
}

fn pci_match_incomplete_pattern(
  id: PCIDeviceID,
  _dev_type: PCIDeviceType,
  pattern: PCIDeviceID,
) -> bool {
  if id.vendor_id == 0xFFFF && id.device_id == 0xFFFF {
    return false;
  }
  if pattern.vendor_id != 0xFFFF && id.vendor_id != pattern.vendor_id {
    return false;
  }
  if pattern.device_id != 0xFFFF && id.device_id != pattern.device_id {
    return false;
  }
  true
}

fn pci_matches_pattern_by_devaddr(devaddr: u32, find: PCIDeviceID) -> bool {
  let id = get_device_id(devaddr);
  if id.vendor_id == 0xFFFF && id.device_id == 0xFFFF {
    return false;
  }
  let dev_type = get_device_type(devaddr);
  pci_match_incomplete_pattern(id, dev_type, find)
}

fn pci_search_for_devices_on_bus(bus: u8, find: PCIDeviceID, last: Option<u32>) -> u32 {
  let mut found_any_device = false;
  let mut next_device = 0u32;

  for slot in 0..32 {
    let mut num_func = 1u32;
    let mut func = 0;
    while func < num_func {
      let devaddr = make_dev_addr(bus, slot, func as u8);
      if last.unwrap_or(0u32) < devaddr
        && (!found_any_device || devaddr < next_device)
        && pci_matches_pattern_by_devaddr(devaddr, find)
      {
        next_device = devaddr;
        found_any_device = true;
      }
      let header = unsafe { pci_read8(devaddr, PCIFIELD_HHEADER_TYPE) };
      if header & 0x80 == 0x80 {
        num_func = 8;
      }
      if (header & 0x7f) == 0x1 {
        let sub_bus_id = unsafe { pci_read8(devaddr, PCIFIELD_SECONDARY_BUS_NUMBER) };
        let rec_ret = pci_search_for_devices_on_bus(sub_bus_id, find, last);
        if last.unwrap_or(0) < rec_ret && (!found_any_device || rec_ret < next_device) {
          next_device = rec_ret;
          found_any_device = true;
        }
      }
      func += 1;
    }
  }

  if !found_any_device {
    return 0;
  }

  return next_device;
}

pub fn search_for_devices(find: PCIDeviceID, last: u32) -> u32 {
  pci_search_for_devices_on_bus(0, find, Some(last))
}

pub fn tmp_init_devs() {
  let vbe_addr = search_for_devices(
    PCIDeviceID {
      vendor_id: 0x1234,
      device_id: 0x1111,
    },
    0,
  );

  println!("[BGE Adapter @ 0x{:08x}] Found", vbe_addr);
}
