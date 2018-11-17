use bootloader::bootinfo::{BootInfo, FrameRange, MemoryMap, MemoryRegion, MemoryRegionType};
use lazy_static::lazy_static;
use spin::Mutex;

use x86_64::{
  structures::paging::{
    FrameAllocator, Mapper, PageTable, PageTableFlags, PhysFrame, PhysFrameRange,
    RecursivePageTable, Size4KiB,
  },
  PhysAddr,
};

pub struct Allocator {
  pub memory_map: &'static mut MemoryMap,
}

impl Allocator {
  fn phys_range(range: FrameRange) -> PhysFrameRange {
    PhysFrameRange {
      start: PhysFrame::from_start_address(PhysAddr::new(range.start_addr())).unwrap(),
      end: PhysFrame::from_start_address(PhysAddr::new(range.end_addr())).unwrap(),
    }
  }

  fn map_range(range: PhysFrameRange) -> FrameRange {
    FrameRange::new(
      range.start.start_address().as_u64(),
      range.end.start_address().as_u64(),
    )
  }

  pub fn allocate_frame(&mut self, region_type: MemoryRegionType) -> Option<PhysFrame> {
    // try to find an existing region of same type that can be enlarged
    let mut iter = self.memory_map.iter_mut().peekable();
    while let Some(region) = iter.next() {
      if region.region_type == region_type {
        if let Some(next) = iter.peek() {
          if next.range.start_frame_number == region.range.end_frame_number
            && next.region_type == MemoryRegionType::Usable
            && !next.range.is_empty()
          {
            let frame = Allocator::phys_range(region.range).end;
            region.range.end_frame_number += 1;
            iter.next().unwrap().range.start_frame_number += 1;
            return Some(frame);
          }
        }
      }
    }

    fn split_usable_region<'a, I>(iter: &mut I) -> Option<(PhysFrame, PhysFrameRange)>
    where
      I: Iterator<Item = &'a mut MemoryRegion>,
    {
      for region in iter {
        if region.region_type != MemoryRegionType::Usable {
          continue;
        }
        if region.range.is_empty() {
          continue;
        }

        let frame = Allocator::phys_range(region.range).start;
        region.range.start_frame_number += 1;
        return Some((frame, PhysFrame::range(frame, frame + 1)));
      }
      None
    }

    let result = if region_type == MemoryRegionType::PageTable {
      // prevent fragmentation when page tables are allocated in between
      split_usable_region(&mut self.memory_map.iter_mut().rev())
    } else {
      split_usable_region(&mut self.memory_map.iter_mut())
    };

    if let Some((frame, range)) = result {
      self.memory_map.add_region(MemoryRegion {
        range: Allocator::map_range(range),
        region_type,
      });
      Some(frame)
    } else {
      None
    }
  }

  /// Marks the passed region in the memory map.
  ///
  /// Panics if a non-usable region (e.g. a reserved region) overlaps with the passed region.
  pub fn mark_allocated_region(&mut self, region: MemoryRegion) {
    for r in self.memory_map.iter_mut() {
      if region.range.start_frame_number >= r.range.end_frame_number {
        continue;
      }
      if region.range.end_frame_number <= r.range.start_frame_number {
        continue;
      }

      if r.region_type != MemoryRegionType::Usable {
        panic!(
          "region {:x?} overlaps with non-usable region {:x?}",
          region, r
        );
      }

      if region.range.start_frame_number == r.range.start_frame_number {
        if region.range.end_frame_number < r.range.end_frame_number {
          // Case: (r = `r`, R = `region`)
          // ----rrrrrrrrrrr----
          // ----RRRR-----------
          r.range.start_frame_number = region.range.end_frame_number;
          self.memory_map.add_region(region);
        } else {
          // Case: (r = `r`, R = `region`)
          // ----rrrrrrrrrrr----
          // ----RRRRRRRRRRRRRR-
          *r = region;
        }
      } else if region.range.start_frame_number > r.range.start_frame_number {
        if region.range.end_frame_number < r.range.end_frame_number {
          // Case: (r = `r`, R = `region`)
          // ----rrrrrrrrrrr----
          // ------RRRR---------
          let mut behind_r = r.clone();
          behind_r.range.start_frame_number = region.range.end_frame_number;
          r.range.end_frame_number = region.range.start_frame_number;
          self.memory_map.add_region(behind_r);
          self.memory_map.add_region(region);
        } else {
          // Case: (r = `r`, R = `region`)
          // ----rrrrrrrrrrr----
          // -----------RRRR---- or
          // -------------RRRR--
          r.range.end_frame_number = region.range.start_frame_number;
          self.memory_map.add_region(region);
        }
      } else {
        // Case: (r = `r`, R = `region`)
        // ----rrrrrrrrrrr----
        // --RRRR-------------
        r.range.start_frame_number = region.range.end_frame_number;
        self.memory_map.add_region(region);
      }
      return;
    }
    panic!("region {:x?} is not a usable memory region", region);
  }
}

impl<'a> FrameAllocator<Size4KiB> for Allocator {
  fn alloc(&mut self) -> Option<PhysFrame<Size4KiB>> {
    self.allocate_frame(MemoryRegionType::PageTable)
  }
}

lazy_static! {
  pub static ref PAGING: Mutex<Paging> = Mutex::new(Paging {
    allocator: None,
    page_table: None,
  });
}

pub struct Paging {
  allocator: Option<Allocator>,
  page_table: Option<RecursivePageTable<'static>>,
}

impl Paging {
  pub fn init(info: &'static mut BootInfo) {
    let table = info.p4_table_addr as *mut PageTable;
    let page_table = Some(RecursivePageTable::new(unsafe { &mut *table }).unwrap());
    let mmap: &'static mut MemoryMap = &mut info.memory_map;
    let paging: &mut Paging = &mut *PAGING.lock();
    paging.allocator = Some(Allocator { memory_map: mmap });
    paging.page_table = page_table;
  }

  pub fn identity_map(
    &mut self,
    start: PhysAddr,
    end: PhysAddr,
    flags: PageTableFlags,
    inclusive: bool,
  ) {
    let mut table = self.page_table.as_mut().unwrap();
    let mut alloc = self.allocator.as_mut().unwrap();
    match inclusive {
      false => {
        let range = PhysFrame::<Size4KiB>::range(
          PhysFrame::from_start_address(start).unwrap(),
          PhysFrame::from_start_address(end).unwrap(),
        );
        for frame in range {
          table.identity_map(frame, flags, alloc).unwrap().flush();
        }
      }
      true => {
        let range = PhysFrame::<Size4KiB>::range_inclusive(
          PhysFrame::from_start_address(start).unwrap(),
          PhysFrame::from_start_address(end).unwrap(),
        );
        for frame in range {
          table.identity_map(frame, flags, alloc).unwrap().flush();
        }
      }
    };
  }
}
