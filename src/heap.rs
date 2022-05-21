use crate::paging::PAGING;
use bootloader::{boot_info::MemoryRegion, boot_info::MemoryRegionKind, BootInfo};
use linked_list_allocator::LockedHeap;
use x86_64::{structures::paging::PageTableFlags, PhysAddr};

/// Find a memory region suitable for the heap
pub fn find_heap_space(bootinfo: &BootInfo) -> (u64, u64) {
    // Initialize the memory region to None
    let mut found_region: Option<&MemoryRegion> = None;

    let size =
        |region: &MemoryRegion| (region.range.end_addr() - region.range.start_addr()) as usize;

    // Iterate over all memory regions
    for region in bootinfo.memory_map.iter() {
        // Test whether the region is usable
        if region.region_type != MemoryRegionKind::Usable {
            continue;
        }

        // Match on the region
        match found_region {
            // Use this new region if it's the first one
            None => found_region = Some(region),

            // Use this new region only if it is bigger
            // than the previously chosen region.
            Some(fr) => {
                if size(region) > size(fr) {
                    found_region = Some(region)
                }
            }
        }
    }

    // Unwrap the region
    let found_region = found_region.expect("Unable to unwrap heap memory region!");

    // Return a triple with the start- and end-addresses and the size
    (
        found_region.range.start_addr() + 1000 * 4096,
        found_region.range.end_addr(),
    )
}

/// Map the heap memory region and initialize the heap allocator
pub fn map_heap(allocator: &LockedHeap, start: u64, end: u64, size: usize) {
    // Identity map the heap memory region
    PAGING.lock().identity_map(
        PhysAddr::new(start),
        PhysAddr::new(end),
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        false,
    );

    // Initialize the heap allocator with the mapped memory region
    unsafe {
        allocator.lock().init(start as usize, size);
    }
}
