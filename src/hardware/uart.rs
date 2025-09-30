use rp_pico::hal;
use hal::pac;

use pac::interrupt;
use core::cell::{RefCell, RefMut};
use critical_section::{CriticalSection, Mutex};
use heapless::spsc::Queue;

use super::GpsUart;
use crate::co::GPS_UART_QUEUE_CAPACITY;

pub(crate) static UART_QUEUE: Mutex<RefCell<Queue<u8, GPS_UART_QUEUE_CAPACITY>>> =
    Mutex::new(RefCell::new(Queue::new()));
pub(crate) static UART_HANDLER: Mutex<RefCell<Option<UartHandler>>> =
    Mutex::new(RefCell::new(None));

pub struct UartHandler {
    uart: GpsUart,
}

impl UartHandler {
    pub fn new(mut uart: GpsUart) -> Self {
        let init_cmd = "$PMTK314,0,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0*29\r\n";
        uart.write_full_blocking(init_cmd.as_bytes());
        uart.enable_rx_interrupt();
        unsafe {
            pac::NVIC::unmask(pac::interrupt::UART0_IRQ);
        }

        Self { uart }
    }
    /// ISR helper - must be called from `UART0_IRQ`
    fn on_irq(&mut self, cs: CriticalSection) {
        let mut buf = [0u8; 8];
        let mut queue = UART_QUEUE.borrow(cs).borrow_mut();
        while let Ok(read) = self.uart.read_raw(&mut buf) {
            if read == 0 {
                break;
            }

            for &byte in &buf[..read] {
                let _ = queue.enqueue(byte); // Drop if full
            }
        }
    }
}

pub fn use_queue<F>(mut f: F)
where
    F: FnMut(RefMut<'_, Queue<u8, GPS_UART_QUEUE_CAPACITY>>),
{
    critical_section::with(|cs| f(UART_QUEUE.borrow(cs).borrow_mut()));
}

#[allow(static_mut_refs)]
#[pac::interrupt]
fn UART0_IRQ() {
    critical_section::with(|cs| {
        if let Some(ref mut u) = *UART_HANDLER.borrow(cs).borrow_mut() {
            u.on_irq(cs);
        }
    })
}
