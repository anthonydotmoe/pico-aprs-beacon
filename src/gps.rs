use core::str;

use defmt::warn;

use crate::app::Shared;
use crate::co::{GPS_MAX_SENTENCE_LEN, GPS_TASK_POLL_INTERVAL_MS};
use crate::hardware::uart::use_queue;
use crate::sched::Tickable;

pub struct GpsTask {
    next_run_at: u64,
    line_buf: heapless::Vec<u8, GPS_MAX_SENTENCE_LEN>,
    discard_line: bool,
}

impl GpsTask {
    pub fn new() -> Self {
        Self {
            next_run_at: 0,
            line_buf: heapless::Vec::new(),
            discard_line: false,
        }
    }

    fn run(&mut self, now: u64, shared: &mut Shared) {
        self.next_run_at = now.saturating_add(GPS_TASK_POLL_INTERVAL_MS);

        use_queue(|mut q| {
            while let Some(b) = q.dequeue() {
                match b {
                    b'\n' => {
                        if !self.discard_line {
                            if let Ok(sentence) = str::from_utf8(self.line_buf.as_slice()) {
                                if shared.nmea.parse(sentence).is_err() {
                                    warn!("gps: failed to parse sentence");
                                }
                            } else {
                                warn!("gps: invalid utf-8 sentence");
                            }
                        }

                        self.line_buf.clear();
                        self.discard_line = false;
                    }
                    b'\r' => {
                        // Ignore carriage returns
                    }
                    _ => {
                        if self.discard_line {
                            continue;
                        }

                        if self.line_buf.push(b).is_err() {
                            self.line_buf.clear();
                            self.discard_line = true;
                            warn!("gps: sentence too long, dropping");
                        }
                    }
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
