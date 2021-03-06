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

extern crate bitflags;
extern crate bootloader;
extern crate linked_list_allocator;
extern crate pc_keyboard;
extern crate pic8259_simple;
extern crate spin;
extern crate x86_64;

#[macro_use]
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

// A macro to write a string to a device.
macro_rules! device_write {
    ($dev:expr, $($arg:tt)*) => {
        device_write!(__formatted $dev, format!($($arg)*));
    };
    (__formatted $dev:expr, $fmt:expr) => {
        (**crate::hal::DEVICE_MANAGER
            .lock()
            .get_device($dev)
            .unwrap()
            .lock())
        .write_bytes(0, $fmt.as_bytes(), $fmt.len());
    };
}

// A macro for kernel-level logging.
macro_rules! log {
    (__ [$($device:expr),*] => $prefix:expr; $fmt:expr) => {{
        $(
            device_write!(__formatted $device, match $device {
                dev if dev.starts_with("com") => {
                    format!(
                        "{filename}\t[{prefix}] {fmt}\r\n",
                        filename=file!(),
                        prefix=$prefix,
                        fmt=$fmt
                    )
                }
                _ => {
                    format!(
                        "[{prefix}] {fmt}\r\n",
                        prefix=$prefix,
                        fmt=$fmt
                    )
                }
            });
        )*
    }};
    (debug: $($arg:tt)*) => (log!(__ ["com1"] => "debug"; format!($($arg)*)));
    ( info: $($arg:tt)*) => (log!(__ ["com1", "tty0"] => "info"; format!($($arg)*)));
    ( warn: $($arg:tt)*) => (log!(__ ["com1", "tty0"] => "warn"; format!($($arg)*)));
    (error: $($arg:tt)*) => (log!(__ ["com1", "tty0"] => "error"; format!($($arg)*)));
    (fault: $($arg:tt)*) => (log!(__ ["com1", "tty0"] => "fault"; format!($($arg)*)));
    ($($arg:tt)*) => (log!(info: $($arg)*));
}

// A macro for printing a string.
macro_rules! print {
    ($($arg:tt)*) => {
        device_write!("tty0", $($arg)*);
    };
}

// A macro for printing a string followed by a newline.
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

use self::vgaterm::{TerminalDevice, VGA_PTR};

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

use self::cmos::{POSTData, CMOS};

// Hardware Abstraction Layer
mod hal;

use self::hal::DEVICE_MANAGER;

// Serial Bus
mod serial;

use self::serial::{SerialDevice, SerialPort};

mod ansi;

//
//
// Main entry point
//
//

#[no_mangle]
#[allow(clippy::empty_loop)]
pub extern "C" fn _start(bootinfo: &'static mut BootInfo) -> ! {
    // Initialize GDT and IDT
    GDT::init();
    IDT::init();

    // Initialize paging and heap allocation
    let (heap_start, heap_end) = find_heap_space(bootinfo);
    Paging::init(bootinfo);
    map_heap(
        &ALLOCATOR,
        heap_start,
        heap_end,
        (heap_end - heap_start) as usize,
    );

    // Initialize devices
    SerialDevice::init("com1", SerialPort::COM1).unwrap();
    log!(debug: "GDT and IDT initialization complete.");
    log!(debug: "Heap initialization complete.");
    TerminalDevice::init("tty0", VGA_PTR);
    log!(debug: "VGA text screen initialization complete.");

    // Print POST status
    print_post_status();

    // Remap the PIC
    PIC8259::init();
    log!(debug: "PIC remapping complete.");

    // Enable interrupts
    x86_64::instructions::interrupts::enable();
    log!(debug: "Interrupts enabled.");

    // Initialize the PS/2 keyboard
    PS2Keyboard::init();
    log!(debug: "Keyboard initialization complete.");

    // Say hello
    println!("Hello from Hydroxide.");

    // Print the current date and time
    let datetime = CMOS::read_date_time();
    println!(
        "The date is {date}, the time is {time}.",
        date = datetime.as_date(),
        time = datetime.as_time(),
    );

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

            dev.set_video_mode(&mode, true);

            #[inline(always)]
            fn get_col(r: u8, g: u8, b: u8) -> u32 {
                (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
            }

            use crate::bga::{GraphicsProvider, TerminalDriver};
            use core::fmt::Write;
            let mut video = VideoDevice::new(&dev, &mode);
            let mut term = TerminalDriver::new(&mut video);
            Write::write_str(&mut term, "Hello World! [\x1b[32mOK\x1b[0m]\n");
            Write::write_str(&mut term, "This should fail! [\x1b[31mFAIL\x1b[0m]\n");
            Write::write_str(
                &mut term,
                "\x1b[44;37mThis simulates a dark BSOD as we have no light colors :(\n",
            );
            Write::write_str(
                &mut term,
                "\x1b[37;1;44mThis simulates a light BSOD as we have light colors :)\n",
            );

            video.flush();
            Some(dev)
        }
        Err(err) => {
            println!(
                "{}
            ",
                err
            );
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
            log!(debug: "POST power supply status: {}", data.power_supply_status());
            log!(debug: "POST cmos checksum status: {}", data.cmos_checksum_status());
            log!(
                debug: "POST cmos config matches: {}",
                data.configuration_match_status()
            );
            log!(
                debug: "POST cmos memory amount matches: {}",
                data.memory_match_status()
            );
            log!(debug: "POST drive health status: {}", data.drive_status());
            log!(debug: "POST time status: {}", data.time_status());
            log!(debug: "POST adapter init status: {}", data.adapter_init_status());
            log!(debug: "POST adapter status: {}", data.adapter_status());
        }
        None => log!(warn: "Unable to fetch POST information."),
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
    (*crate::hal::DEVICE_MANAGER
        .lock()
        .get_device("tty0")
        .unwrap()
        .lock())
        .as_any()
        .downcast_mut::<crate::vgaterm::TerminalDevice>()
        .unwrap()
        .clear();
    println!(" * **KERNEL PANIC");
    if let Some(location) = info.location() {
        println!(" at {}", location);
    }
    if let Some(message) = info.message() {
        println!(
            "    {}
            ",
            message
        );
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
    (*crate::hal::DEVICE_MANAGER
        .lock()
        .get_device("tty0")
        .unwrap()
        .lock())
        .as_any()
        .downcast_mut::<crate::vgaterm::TerminalDevice>()
        .unwrap()
        .clear();
    println!(" * **OUT OF MEMORY");
    loop {
        x86_64::instructions::hlt();
    }
}
