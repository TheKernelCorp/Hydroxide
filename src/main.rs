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

// TODO: Initialize the heap in the main function
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
    Paging::init(bootinfo);
    PIC8259::init();
    x86_64::instructions::interrupts::enable();
    PS2Keyboard::init();

    println!("Hello from Hydroxide.");

    pci::tmp_init_devs();

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
