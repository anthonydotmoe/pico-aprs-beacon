// I2S audio output with double-buffer DMA.
// Produces a constant stream (silence by default) and lets higher layers
// push AFSK samples zero-copy into the ping/pong buffer.

use core::sync::atomic::{AtomicBool, Ordering};

use defmt::println;

use pio::Instruction;
use rp_pico::hal::pio::UninitStateMachine;
use rp_pico::hal::{self, Clock};
use hal::pac;

use pac::PIO0;
use pac::interrupt;
use hal::clocks::ClocksManager;
use hal::dma::{single_buffer::{self, Transfer}, Channel, Channels, SingleChannel, CH0};
use hal::pio::{PIO, PIOBuilder, ShiftDirection, Tx, SM0, PinDir};

use super::ptt::PttWrapper;

pub type Sample = u32;
pub const BUF_LEN: usize = 256;
pub const SAMPLE_RATE: u32 = 8_000;

// Ping-pong sample storage
pub static mut BUF_PING: [Sample; BUF_LEN] = [0; BUF_LEN];
pub static mut BUF_PONG: [Sample; BUF_LEN] = [0; BUF_LEN];
static ZERO_BUF:         [Sample; BUF_LEN] = [0; BUF_LEN];

pub struct AudioOut {
    xfer: Option<DmaTransfer>,
    use_ping: bool,
    dma_busy: AtomicBool,
    ptt_pin: PttWrapper,
}

#[allow(static_mut_refs)]
impl AudioOut {
    /// Creates the PIO program, state-machine, DMA channel and launches the
    /// first silent transfer.
    pub fn new(
        mut dma: Channels,
        mut pio: PIO<PIO0>,
        sm: UninitStateMachine<(PIO0, SM0)>,
        ptt_pin: PttWrapper,
        clocks: &ClocksManager,
    ) -> Self {
        // 1. Set up PIO program
        let program_with_defines = pio_proc::pio_file!("./src/audio_i2s.pio");
        let entry_point = program_with_defines.public_defines.entry_point as u8;
        let program = program_with_defines.program;

        let installed = pio.install(&program).unwrap();

        let offset = installed.offset();

        let (mut sm, _, tx) = PIOBuilder::from_installed_program(installed)
            .out_pins(20, 1)
            .side_set_pin_base(21)
            .out_shift_direction(ShiftDirection::Left)
            .autopull(true)
            .pull_threshold(32)
            .build(sm);

        sm.set_pindirs([
            (20, PinDir::Output),
            (21, PinDir::Output),
            (22, PinDir::Output),
        ]);

        // 8 kHz * 16 bits * 2 (stereo) * 2 (both edges)
        let bclk = SAMPLE_RATE * 16 * 2 * 2;
        let sys_clk = clocks.system_clock.freq().to_Hz();
        let divisor = sys_clk as f32 / bclk as f32;

        println!("BCLK: {}\nSYS_CLK: {}\ndivisor: {}", bclk, sys_clk, divisor);

        sm.set_clock_divisor(divisor);

        // Jump to entry_point
        let jmp = Instruction {
            operands: pio::InstructionOperands::JMP { condition: pio::JmpCondition::Always, address: offset + entry_point },
            delay: 0,
            side_set: Some(0)
        };

        sm.exec_instruction(jmp);
        sm.start();
        // 2. Set up DMA channel
        // Enable IRQ
        dma.ch0.enable_irq0();

        // First silent transfer so the PIO can begin
        let xfer = single_buffer::Config::new(dma.ch0, &ZERO_BUF, tx).start();

        unsafe { pac::NVIC::unmask(pac::interrupt::DMA_IRQ_0) };

        Self {
            xfer: Some(xfer),
            use_ping: true,
            dma_busy: AtomicBool::new(true),
            ptt_pin,
        }

    }

    /// Returns a `&mut` slice for the *inactive* buffer so upper-layers can
    /// write samples directly.
    pub fn get_free_buffer(&mut self) -> &'static mut [Sample; BUF_LEN] {
        unsafe {
            if self.use_ping {
                &mut BUF_PING
            } else {
                &mut BUF_PONG
            }
        }
    }

    /// Queues the *just-filled* buffer for DMA once the current transfer
    /// finishes. Called from main-loop after filling.
    pub fn queue_filled(&mut self) {
        self.dma_busy.store(true, Ordering::SeqCst);
    }

    /// ISR helper - must be called from `DMA_IRQ_0`
    pub fn on_dma_complete(&mut self) {
        let (mut ch, _buf, pio_tx) = self.xfer.take().unwrap().wait();
        if !ch.check_irq0() {
            return;
        }

        let was_busy = self.dma_busy.load(Ordering::Relaxed);
        self.dma_busy.store(false, Ordering::Relaxed);

        let next = if was_busy {
            if self.use_ping {
                self.ptt_pin.key(true);
                unsafe { &BUF_PING }
            } else {
                self.ptt_pin.key(true);
                unsafe { &BUF_PONG }
            }
        } else {
            self.ptt_pin.key(false);
            &ZERO_BUF
        };

        self.use_ping = !self.use_ping;

        self.dma_busy.store(false, Ordering::SeqCst);

        // Launch next transfer
        self.xfer = Some(single_buffer::Config::new(ch, next, pio_tx).start());
    }

    /// Non-blocking query so main-loop knows when it may write next chunk
    pub fn is_busy(&self) -> bool {
        self.dma_busy.load(Ordering::SeqCst)
    }
}

//------------------------------------------------------------------------------
// Global singletons to bootstrap DMA_IRQ_0:
// DMA_XFER - Information about the first transfer, used to call `.wait()`
// AUDIO_OUT - `self`

use core::cell::RefCell;
use critical_section::Mutex;

type DmaTransfer = Transfer<Channel<CH0>, &'static [Sample; BUF_LEN], Tx<(PIO0, SM0)>>;

pub(crate) static AUDIO_OUT: Mutex<RefCell<Option<AudioOut>>> = Mutex::new(RefCell::new(None));

#[allow(static_mut_refs)]
#[pac::interrupt]
fn DMA_IRQ_0() {
    critical_section::with(|cs| {
        if let Some(ref mut audio) = *AUDIO_OUT.borrow(cs).borrow_mut() {
            audio.on_dma_complete();
        }
    });
}

// Public facade for accessing the singleton -----------------------------------

pub fn is_busy() -> bool {
    critical_section::with(|cs| {
        AUDIO_OUT
            .borrow(cs)
            .borrow()
            .as_ref()
            .map(|a| a.is_busy())
            .unwrap_or(true)    // Treat uninitialized as busy
    })
}

pub fn free_buffer() -> Option<&'static mut [Sample; BUF_LEN]> {
    critical_section::with(|cs| {
        AUDIO_OUT
            .borrow(cs)
            .borrow_mut()
            .as_mut()
            .map(|a| a.get_free_buffer())
    })
}

pub fn queue_filled() {
    critical_section::with(|cs| {
        if let Some(a) = AUDIO_OUT.borrow(cs).borrow_mut().as_mut() {
            a.queue_filled();
        }
    });
}
