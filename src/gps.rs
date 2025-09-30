use defmt::println;

use crate::app::Shared;
use crate::hardware::uart::use_queue;
use crate::sched::Tickable;
use crate::co::UART_BUFFER_SIZE;


pub struct GpsTask {
    next_run_at: u64,
    line_buf: heapless::String<UART_BUFFER_SIZE>,
}

impl GpsTask {
    pub fn new() -> Self {
        Self {
            next_run_at: 0,
            line_buf: heapless::String::new(),
        }
    }

    fn run(&mut self, now: u64, shared: &mut Shared) {
        self.next_run_at = now + 900; // 1 second + a little more

        use_queue(|mut q| {
            println!("Queue: {}", q.len());
            while let Some(b) = q.dequeue() {
                if b == b'\n' {
                    // Full line acquired
                    let head = &self.line_buf[..10];

                    if let Err(_) = shared.nmea.parse(self.line_buf.as_str()) {
                        println!("Error with: \"{}\"", head);
                    }

                    self.line_buf.clear();
                } else if b != b'\r' {
                    let _ = self.line_buf.push(b as char);
                }
            }
        });
    }
}

impl Tickable for GpsTask {
    fn next_run_at(&self) -> u64 {
        self.next_run_at
    }
    fn tick(&mut self, now: u64, shared: &mut Shared) {
        self.run(now, shared)
    }
}

//-------------
