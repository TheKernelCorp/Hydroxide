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
#![feature(asm)]
#![feature(naked_functions)]
#![feature(thread_local)]

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

extern crate alloc;
extern crate bitflags;
extern crate bootloader;
extern crate linked_list_allocator;
extern crate pc_keyboard;
extern crate pic8259_simple;
extern crate spin;
extern crate x86_64;

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
        core::fmt::Write::write_fmt(
            (*crate::hal::DEVICE_MANAGER
                .lock()
                .get_device("tty0")
                .unwrap()
                .lock()
            ).as_any()
            .downcast_mut::<crate::vgaterm::TerminalDevice>()
            .unwrap(),
            format_args!($($arg)*)
        ).unwrap();
    };
}

macro_rules! println {
    () => (print ! ("\n"));
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

// CMOS
mod cmos;

use self::cmos::{POSTData, CMOS};

// Hardware Abstraction Layer
mod hal;

use self::hal::DEVICE_MANAGER;

// Serial Bus
mod serial;

use self::serial::{SerialDevice, SerialPort};

mod context;

//
//
// Main entry point
//
//

pub extern "C" fn test() {
    /*// Initialize the PS/2 keyboard
    PS2Keyboard::init();
    // Print the current date and time
    let datetime = CMOS::read_date_time();
    println!(
        "The date is {date}, the time is {time}.",
        date = datetime.as_date(),
        time = datetime.as_time(),
    );

    // Say hello
    println!("Hello from Hydroxide.");*/

    loop {
        print!("a");
        x86_64::instructions::hlt();
    }
}

pub extern "C" fn test2() {
    //println!("Hello from the second task!");

    loop {
        print!("c");
        x86_64::instructions::hlt();
    }
}

#[no_mangle]
#[allow(clippy::empty_loop)]
pub unsafe extern "C" fn _start(bootinfo: &'static mut BootInfo) -> ! {
    let mut stack = None;
    for map in bootinfo.memory_map.iter() {
        if map.region_type == bootloader::bootinfo::MemoryRegionType::KernelStack {
            stack = Some(map.range);
        }
    }

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
    SerialDevice::init("com1", SerialPort::COM1);
    TerminalDevice::init("tty0", VGA_PTR);

    // Print POST status
    print_post_status();

    context::init(
        (stack.unwrap().start_frame_number * 4096) as usize,
        (stack.unwrap().start_frame_number * 4096) as usize
            - (stack.unwrap().end_frame_number * 4096) as usize,
    );

    match context::contexts_mut().spawn(test) {
        Ok(context) => context.write().status = context::context::Status::Runnable,
        _ => {}
    };

    match context::contexts_mut().spawn(test2) {
        Ok(context) => context.write().status = context::context::Status::Runnable,
        _ => {}
    };

    // Remap the PIC
    PIC8259::init();

    // Enable interrupts
    x86_64::instructions::interrupts::enable();

    // Idle
    loop {
        print!("x");
        x86_64::instructions::hlt();
    }
}

fn print_post_status() {
    match CMOS::read_post_data() {
        Some(data) => {
            println!("[post] power supply status: {}", data.power_supply_status());
            println!(
                "[post] cmos checksum status: {}",
                data.cmos_checksum_status()
            );
            println!(
                "[post] cmos config matches: {}",
                data.configuration_match_status()
            );
            println!(
                "[post] cmos memory amount matches: {}",
                data.memory_match_status()
            );
            println!("[post] drive health status: {}", data.drive_status());
            println!("[post] time status: {}", data.time_status());
            println!("[post] adapter init status: {}", data.adapter_init_status());
            println!("[post] adapter status: {}", data.adapter_status());
        }
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
    /*(*crate::hal::DEVICE_MANAGER
        .lock()
        .get_device("tty0")
        .unwrap()
        .lock())
    .as_any()
    .downcast_mut::<crate::vgaterm::TerminalDevice>()
    .unwrap()
    .clear();*/
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
    (*crate::hal::DEVICE_MANAGER
        .lock()
        .get_device("tty0")
        .unwrap()
        .lock())
    .as_any()
    .downcast_mut::<crate::vgaterm::TerminalDevice>()
    .unwrap()
    .clear();
    println!("*** OUT OF MEMORY");
    loop {
        x86_64::instructions::hlt();
    }
}
