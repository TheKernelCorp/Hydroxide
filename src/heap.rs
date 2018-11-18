use bootloader::bootinfo::{
    BootInfo,
    MemoryRegion,
    MemoryRegionType,
};
use x86_64::{
    PhysAddr,
    structures::paging::PageTableFlags,
};
use linked_list_allocator::LockedHeap;
use crate::paging::PAGING;

/// Find a memory region suitable for the heap
pub fn find_heap_space(bootinfo: &BootInfo) -> (u64, u64, usize) {

    // Initialize the memory region to None
    let mut found_region: Option<&MemoryRegion> = None;

    /// Determine the size of the specified memory region
    fn size(region: &MemoryRegion) -> usize {
        (region.range.end_addr() - region.range.start_addr()) as usize
    }

    // Iterate over all memory regions
    for region in bootinfo.memory_map.iter() {

        // Test whether the region is usable
        if region.region_type != MemoryRegionType::Usable {
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
        found_region.range.start_addr(),
        found_region.range.end_addr(),
        size(found_region),
    )
}

/// Map the heap memory region and initialize the heap allocator
pub fn map_heap(allocator: &LockedHeap, start: u64, end: u64, size: usize) {

    // Print information about the heap memory region
    println!(
        "[heap] start: 0x{:08x}; end: 0x{:08x}; size: 0x{:08x}",
        start, end, size
    );

    // Identity map the heap memory region
    PAGING.lock().identity_map(
        PhysAddr::new(start),
        PhysAddr::new(end),
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
        false,
    );

    // Initialize the heap allocator with the mapped memory region
    unsafe {
        allocator.lock().init(start as usize, size);
    }
}
