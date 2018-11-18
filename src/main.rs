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
extern crate pc_keyboard;
extern crate pic8259_simple;
extern crate spin;
extern crate x86_64;
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

// Programmable Interrupt Controller
// Intel 8259
mod pic;

use self::pic::PIC8259;

// Programmable Interrupt Timer
// Intel 825x
mod pit;

// VGA Terminal Screen Buffer
mod vgaterm;

// Generic PS/2 Keyboard
mod ps2kbd;

use self::ps2kbd::PS2Keyboard;

mod pci;

mod paging;

use self::paging::Paging;

mod bga;

use self::bga::BochsGraphicsAdapter;

mod heap;

use self::heap::{find_heap_space, map_heap};

//
//
// Main entry point
//
//

#[no_mangle]
#[allow(clippy::empty_loop)]
pub extern "C" fn _start(bootinfo: &'static mut BootInfo) -> ! {
    GDT::init();
    IDT::init();

    let (heap_start, heap_end, heap_size) = find_heap_space(bootinfo);
    Paging::init(bootinfo);

    map_heap(&ALLOCATOR, heap_start, heap_end, heap_size);

    PIC8259::init();
    x86_64::instructions::interrupts::enable();
    PS2Keyboard::init();

    println!("Hello from Hydroxide.");

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

            dev.set_video_mode(&mode, true);

            fn get_col(r: u8, g: u8, b: u8) -> u32 {
                ((r as u32) << 16) | ((g as u32) << 8) | ((b as u32) << 0)
            }

            let mut fb = dev.get_framebuffer(&mode);
            for y in 0..mode.height {
                for x in 0..mode.width {
                    unsafe {
                        let c = x as u8 ^ y as u8;
                        fb.as_mut()[x + y * mode.width] = get_col(c, c, c);
                    }
                }
            }

            Some(dev)
        }
        Err(err) => {
            println!("{}", err);
            None
        }
    }
        .unwrap();

    loop {
        x86_64::instructions::hlt();
    }
}

//
//
// Panic and OOM handlers
//
//

/// This function is called on panic.
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
#[alloc_error_handler]
#[no_mangle]
pub extern "C" fn oom(_: ::core::alloc::Layout) -> ! {
    vgaterm::KTERM.lock().clear();
    println!("*** OUT OF MEMORY");
    loop {
        x86_64::instructions::hlt();
    }
}
