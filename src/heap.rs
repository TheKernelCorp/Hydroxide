use bootloader::bootinfo::{BootInfo, MemoryRegion, MemoryRegionType};
use linked_list_allocator::LockedHeap;

use crate::paging::PAGING;

pub fn find_heap_space(bootinfo: &BootInfo) -> (u64, u64, usize) {
  let mut found_region: Option<&MemoryRegion> = None;
  fn size(region: &MemoryRegion) -> usize {
    (region.range.end_addr() - region.range.start_addr()) as usize
  }
  for region in bootinfo.memory_map.iter() {
    if region.region_type == MemoryRegionType::Usable {
      match found_region {
        None => found_region = Some(region),
        Some(fr) => {
          if size(region) > size(fr) {
            found_region = Some(region)
          }
        }
      }
    }
  }
  let found_region = found_region.unwrap();

  (
    found_region.range.start_addr(),
    found_region.range.end_addr(),
    size(found_region),
  )
}

pub fn map_heap(allocator: &LockedHeap, start: u64, end: u64, size: usize) {
  use x86_64::structures::paging::PageTableFlags;
  use x86_64::PhysAddr;

  println!(
    "Initialized heap at 0x{:08x} end 0x{:08x} | size: 0x{:08x}",
    start, end, size
  );

  PAGING.lock().identity_map(
    PhysAddr::new(start),
    PhysAddr::new(end),
    PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
    true,
  );

  unsafe {
    allocator.lock().init(start as usize, size);
  }
}
