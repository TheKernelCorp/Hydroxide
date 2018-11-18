//
// no_std
//
// Inform the compiler that we are
// writing code for a freestanding
// environment without std support.
//
#![no_std]
//
// no_main
//
// Make the compiler happy about there
// not being a main function.
//
#![no_main]
//
// Enable features
//

//
// Enable the x86 Interrupt ABI
//
// This is needed for writing interrupt
// handlers in pure Rust without having
// to resort to naked function trickery.
//
#![feature(abi_x86_interrupt)]
//
// Enable pointer internals
//
// I'd very much like to get rid of this
// later on, but for now it's needed for
// Unique pointer support.
//
#![feature(ptr_internals)]
//
// Enable allocation error handlers
//
// This is needed for handling OOM scenarios
// with a custom memory allocator.
//
#![feature(alloc_error_handler)]
//
// Enable panic info messages
//
// This feature allows for the use of associated
// panic messages. That way, we can display nice
// and helpful messages in case of kernel panics.
//
#![feature(panic_info_message)]

#![feature(alloc)]

#![feature(extern_crate_item_prelude)]

#![feature(box_syntax)]

#![feature(raw_vec_internals)]

//
// Import crates
//
// This is technically not needed, since we
// are using Rust 2018 edition.
//
// However, for some reason, the RLS still doesn't
// populate autocomplete properly if the extern crate
// imports are not there.
//

extern crate bootloader;
extern crate linked_list_allocator;
extern crate pic8259_simple;
extern crate spin;
extern crate x86_64;
extern crate pc_keyboard;
extern crate bitflags;
extern crate alloc;

//
//
// Import structures
//
//

use bootloader::bootinfo::BootInfo;

use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;

//
//
// Add print and println macro support
//
//

macro_rules! print {
    ($($arg:tt)*) => {
        core::fmt::Write::write_fmt(&mut *crate::vgaterm::KTERM.lock(), format_args!($($arg)*)).unwrap();
    };
}

macro_rules! println {
    () => (print!("\n"));
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

//
//
// Provide a global allocator
//
//

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

//
//
// Load kernel components
//
//

// Global Descriptor Table
mod gdt;
use self::gdt::GDT;

// Interrupt Descriptor Table
// Task State Segment
mod idt;
use self::idt::IDT;

// Intel 8259
// Programmable Interrupt Controller
mod pic;
use self::pic::PIC8259;

// Intel 825x
// Programmable Interrupt Timer
mod pit;

// VGA Terminal Screen Buffer
mod vgaterm;

// Intel 8042
// Keyboard Controller
mod kbc;

// Generic PS/2 Keyboard
mod ps2kbd;
use self::ps2kbd::PS2Keyboard;

// Page Allocator
mod paging;
use self::paging::Paging;

// Heap Allocator
mod heap;
use self::heap::{find_heap_space, map_heap};

// Peripheral Component Interconnect
mod pci;

// Bochs Graphics Adapter
mod bga;
use self::bga::{BochsGraphicsAdapter, VideoDevice};

// CMOS
mod cmos;
use self::cmos::{
    CMOS,
    POSTData,
};

//
//
// Main entry point
//
//

pub fn map_free_region(bootinfo: &BootInfo) -> (u64, u64) {
  use bootloader::bootinfo::{MemoryRegion,MemoryRegionType};
  let size = |region: &MemoryRegion|
    (region.range.end_addr() - region.range.start_addr()) as usize;
  for region in bootinfo.memory_map.iter() {
    let sz: u64 = 4097;
    if region.region_type != MemoryRegionType::Usable { continue }
    if size(region) < sz as usize { continue }
    return (region.range.start_addr(), region.range.start_addr() + sz);
  }
  panic!("Error.")
}

#[no_mangle]
#[allow(clippy::empty_loop)]
pub extern "C" fn _start(bootinfo: &'static mut BootInfo) -> ! {

    // Initialize GDT and IDT
    GDT::init();
    IDT::init();

    // Print POST status
    print_post_status();


    // Initialize paging and heap allocation
    let (heap_start, heap_end, heap_size) = find_heap_space(bootinfo);
    let region = map_free_region(bootinfo);
    Paging::init(bootinfo);
    use x86_64::{PhysAddr, structures::paging::PageTableFlags};
    crate::paging::PAGING.lock().identity_map(
      PhysAddr::new(region.0),
      PhysAddr::new(region.1),
      PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
      true,
    );
    // map_heap(&ALLOCATOR, heap_start, heap_end, heap_size);

    // Remap the PIC
    PIC8259::init();

    // Enable interrupts
    x86_64::instructions::interrupts::enable();

    // Initialize the PS/2 keyboard
    PS2Keyboard::init();

    // Print the current date and time
    let datetime = CMOS::read_date_time();
    println!(
        "The date is {date}, the time is {time}.",
        date = datetime.as_date(),
        time = datetime.as_time(),
    );

    // Say hello
    println!("Hello from Hydroxide.");
    use alloc::vec;
    // let x = vec![0u32; 2048];
    loop {
        x86_64::instructions::hlt();
    }

    // Detect a Bochs Graphics Adapter
    let bga = match BochsGraphicsAdapter::detect() {
        Ok(device) => {
            let mut dev = BochsGraphicsAdapter::new(&device).init();
            println!("[BGA @ 0x{:08x}] Found", dev.addr());
            println!(
                "[BGA @ 0x{:08x}] Version: 0x{:04x}",
                dev.addr(),
                dev.version()
            );
            println!("[BGA @ 0x{:08x}] Max BPP: {}", dev.addr(), dev.max_bpp);
            println!("[BGA @ 0x{:08x}] Max Width: {}", dev.addr(), dev.max_width);
            println!(
                "[BGA @ 0x{:08x}] Max Height: {}",
                dev.addr(),
                dev.max_height
            );
            let mode = dev
                .get_default_mode()
                .and_then(|mode| {
                    println!(
                        "[BGA @ 0x{:08x}] Supports resolution: {}x{}x{}",
                        dev.addr(),
                        mode.width,
                        mode.height,
                        mode.bpp
                    );
                    Some(mode)
                })
                .unwrap();
            
            // dev.set_video_mode(&mode, true);

            #[inline(always)]
            fn get_col(r: u8, g: u8, b: u8) -> u32 {
                (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
            }

            use crate::bga::GraphicsProvider;
            // dev.get_framebuffer(&mode)[0] = 0xFFFF_FFFF;
            let mut video = VideoDevice::new(&dev, &mode);
            // for y in 0..mode.height {
            //     for x in 0..mode.width {
            //         unsafe {
            //             let c = (x % 0xFF) as u8 ^ (y % 0xFF) as u8;
            //             video.buffer[x + y * mode.width] = get_col(c, c, c);
            //         }
            //     }
            // }

            // video.flush();
            Some(dev)
        }
        Err(err) => {
            println!("{}", err);
            None
        }
    }
    .unwrap();

    // Idle
    loop {
        x86_64::instructions::hlt();
    }
}

fn print_post_status() {
    match CMOS::read_post_data() {
        Some(data) => {
            println!("[post] power supply status: {}", data.power_supply_status());
            println!("[post] cmos checksum status: {}", data.cmos_checksum_status());
            println!("[post] cmos config matches: {}", data.configuration_match_status());
            println!("[post] cmos memory amount matches: {}", data.memory_match_status());
            println!("[post] drive health status: {}", data.drive_status());
            println!("[post] time status: {}", data.time_status());
            println!("[post] adapter init status: {}", data.adapter_init_status());
            println!("[post] adapter status: {}", data.adapter_status());
        },
        None => println!("[post] unable to fetch POST information."),
    };
}

//
//
// Panic and OOM handlers
//
//

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
#[allow(clippy::empty_loop)]
fn panic(info: &PanicInfo) -> ! {
    vgaterm::KTERM.lock().clear();
    println!("*** KERNEL PANIC");
    if let Some(location) = info.location() {
        println!(" at {}", location);
    }
    if let Some(message) = info.message() {
        println!("    {}", message);
    } else {
        println!("Unknown cause.");
    }
    loop {
        x86_64::instructions::hlt();
    }
}

/// This function is called on allocation error.
#[cfg(not(test))]
#[alloc_error_handler]
#[no_mangle]
pub extern "C" fn oom(_: ::core::alloc::Layout) -> ! {
    vgaterm::KTERM.lock().clear();
    println!("*** OUT OF MEMORY");
    loop {
        x86_64::instructions::hlt();
    }
}
