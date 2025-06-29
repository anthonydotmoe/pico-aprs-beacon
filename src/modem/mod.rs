use crate::hardware::audio;
use crate::sched::Tickable;

include!(concat!(env!("OUT_DIR"), "/sine_table.rs")); // Imports SINE_TABLE

// Constants
use audio::SAMPLE_RATE;
const TABLE_SIZE: u32 = SINE_TABLE.len() as u32;
const SCALE: u32 = 1 << 16;

const fn phase_step(freq: u32) -> u32 {
    ((freq as u64 * TABLE_SIZE as u64 * SCALE as u64) / SAMPLE_RATE as u64) as u32
}

const PHASE_STEP_MARK:  u32 = phase_step(1200);
const PHASE_STEP_SPACE: u32 = phase_step(2200);

const BUFFERS_PER_STATE: usize = 100;

enum OutputState {
    Tone,
    Silent,
}

struct SinGenerator {
    phase: u32, // 16.16 fixed-point
    count: usize,
    bit: bool,

    next_tx_time: u64,

    state: OutputState,
    state_counter: usize,
}

const BIT_RATE: usize = SAMPLE_RATE as usize / 1200;

impl SinGenerator {
    pub fn new() -> Self {
        Self {
            phase: 0,
            count: BIT_RATE,
            bit: false,
            
            next_tx_time: 0,

            state: OutputState::Tone,
            state_counter: 0,
        }
    }
    pub fn run(&mut self, now: u64) {

        // If DMA is busy, yield immediately
        if audio::is_busy() {
            self.next_tx_time = now + 1;
            return;
        }

        match self.state {
            OutputState::Tone => {
                let buf = match audio::free_buffer() {
                    Some(buf) => buf,
                    None => return,
                };

                for sample in buf.iter_mut() {
                    if self.count != 0 {
                        self.count = self.count - 1;
                    } else {
                        self.count = BIT_RATE;
                        self.bit = !self.bit;
                    }

                    let step = if self.bit { PHASE_STEP_MARK } else { PHASE_STEP_SPACE };
                    self.phase = self.phase.wrapping_add(step);
                    let index = ((self.phase >> 16) & (TABLE_SIZE - 1)) as usize;
                    *sample = SINE_TABLE[index];
                }
                audio::queue_filled();
                self.state_counter += 1;
            }
            OutputState::Silent => {
                self.next_tx_time = now + 1000;
                self.state_counter = BUFFERS_PER_STATE;
                // Skip everything, let zero fallback engage
            }
        }

        if self.state_counter >= BUFFERS_PER_STATE {
            self.state_counter = 0;
            self.state = match self.state {
                OutputState::Silent => OutputState::Tone,
                OutputState::Tone => OutputState::Silent,
            };
        }
    }
}

impl Tickable for SinGenerator {
    fn next_run_at(&self) -> u64 {
        self.next_tx_time
    }

    fn tick(&mut self, now: u64) {
        self.run(now);
    }

}