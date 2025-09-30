use crate::app::Shared;
use crate::ax25::TxBits;
use crate::hardware::audio;
use crate::sched::Tickable;

include!(concat!(env!("OUT_DIR"), "/sine_table.rs")); // Imports SINE_TABLE

// Constants
use audio::SAMPLE_RATE;
use cortex_m::asm::wfi;
const TABLE_SIZE: u32 = SINE_TABLE.len() as u32;
const PHASE_FRAC: u32 = 1 << 16;

const fn phase_step(freq: u32) -> u32 {
    ((freq as u64 * TABLE_SIZE as u64 * PHASE_FRAC as u64) / SAMPLE_RATE as u64) as u32
}

const STEP_MARK:  u32 = phase_step(1200);
const STEP_SPACE: u32 = phase_step(2200);

// Q16.16 samples per bit
const BITS_PER_SAMPLE_Q16: u32 = ((1200u64 << 16) / (SAMPLE_RATE as u64)) as u32;
const ONE_Q16: u32 = 1 << 16;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum AfskTone { Mark, Space }

pub struct AfskModulator {
    phase: u32,     // 16.16
    bit_accum: u32, // 16.16
    nrzi_level: bool,
    tone: AfskTone,

    // current source
    src: Option<TxBits>,

    next_run: u64,
}

impl AfskModulator {
    pub fn new() -> Self {
        Self {
            phase: 0,
            bit_accum: 0,
            nrzi_level: false,
            tone: AfskTone::Mark,
            src: None,
            next_run: 0,
        }
    }

    fn load_next(&mut self, shared: &mut Shared) -> bool {
        if self.src.is_none() {
            if let Some(next) = shared.txq.pop_front() {
                self.src = Some(next);
            }
        }
        self.src.is_some()
    }

    fn fill_one_buffer(&mut self) -> bool {
        let Some(buf) = audio::free_buffer() else { return true; };

        let mut tx_active = self.src.is_some();
        for s in buf.iter_mut() {
            // Bit clock: advance by one sample
            self.bit_accum = self.bit_accum.wrapping_add(BITS_PER_SAMPLE_Q16);

            // one bit elapsed?
            if self.bit_accum >= ONE_Q16 {
                self.bit_accum -= ONE_Q16;

                if let Some(src) = self.src.as_mut() {
                    if let Some(bit) = src.pull_bit() {
                        if !bit { self.nrzi_level = !self.nrzi_level; } // 0 => toggle
                    } else {
                        // Source exhausted. Drop it and force MARK thereafter
                        self.src = None;
                        self.nrzi_level = false; // MARK
                        tx_active = false;
                    }
                } else {
                    // After frame end (or idle), hold MARK
                    self.nrzi_level = false;
                }

                self.tone = if self.nrzi_level { AfskTone::Space } else { AfskTone::Mark };
            }

            // Tone DDS
            let step = match self.tone { AfskTone::Mark => STEP_MARK, AfskTone::Space => STEP_SPACE };
            self.phase = self.phase.wrapping_add(step);
            let index = ((self.phase >> 16) & (TABLE_SIZE - 1)) as usize;
            *s = SINE_TABLE[index];
        }

        audio::queue_filled();
        tx_active
    }

    fn transmit_blocking(&mut self, shared: &mut Shared) {
        // Try to start with a frame; if none queued, return immediately
        if !self.load_next(shared) {
            return;
        }

        loop {
            // Wait until the audio DMA can accept a buffer
            while audio::is_busy() {
                // Let ISRs run (DMA, UART, timer), but don't yield to other tasks
                wfi();
            }

            // FIll and queue a buffer
            let still_tx = self.fill_one_buffer();

            // If the current frame ended inside this buffer, see if there's another frame queued
            if !still_tx {
                if !self.load_next(shared) {
                    // No more frames
                    break;
                }
            }
            // else: continue; frame still active
        }
    }

    pub fn run(&mut self, now: u64, shared: &mut Shared) {
        // If there is work in the queue (or a frame in progress), hog the CPU and run to completion
        if self.src.is_some() || !shared.txq.is_empty() {
            self.transmit_blocking(shared);
            self.next_run = now + 1;
            return;
        }

        // Nothing to do; stay quiet.
        self.next_run = now + 100;
    }
}

impl Tickable for AfskModulator {
    fn next_run_at(&self) -> u64 {
        self.next_run
    }

    fn tick(&mut self, now: u64, shared: &mut Shared) {
        self.run(now, shared);
    }

}
