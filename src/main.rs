#![no_std]
#![no_main]

extern crate alloc;

use defmt::println;
#[cfg(feature = "alloc")]
use embedded_alloc::LlffHeap as Heap;

use rp_pico as bsp;

use bsp::entry;
//use defmt::*;
use defmt_rtt as _;

// Panic handler
#[cfg(debug_assertions)]
use panic_probe as _;
#[cfg(not(debug_assertions))]
use panic_halt as _;

use bsp::hal;
use hal::pac;

mod app;
mod aprs;
mod ax25;
mod beacon;
mod bitstream;
mod display;
mod gps;
mod hardware;
mod modem;
mod sched;


// TODO: Figure out why I need this `global_allocator`

/*
#[cfg(feature = "alloc")]
#[global_allocator]
static HEAP: Heap = Heap::empty();
*/

#[global_allocator]
static ALLOC: Dumb = Dumb {};

struct Dumb {}

use core::{alloc::GlobalAlloc, ptr::null_mut};
unsafe impl GlobalAlloc for Dumb {
    unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
        panic!("You called alloc!");
        null_mut()
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

pub mod co {
    pub const MYCALL: &'static str = "KI7TUK-1";
    pub const TOCALL: &'static str = "APZ   ";
    pub const UART_BUFFER_SIZE: usize = 4096;
}

// Entry point
#[entry]
fn main() -> ! {
    /*
    #[cfg(feature = "alloc")]
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(core::ptr::addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE)}
    }
    */
    // Get singleton objects
    let pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    app::run(pac, core);
}

// Binary Info section ---------------------------------------------------------

use rp_binary_info as binary_info;

#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [binary_info::EntryAddr; 2] = [
    binary_info::rp_program_name!(c"Pico APRS Beacon"),
    binary_info::rp_cargo_version!(),
];