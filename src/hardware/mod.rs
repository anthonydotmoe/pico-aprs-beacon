use rp_pico::hal;
use rp_pico::hal::dma::DMAExt;
use rp_pico::hal::pac;

use hal::clocks::Clock;
use hal::gpio::{self, Pins, FunctionPio0};
use hal::sio::Sio;
use hal::timer::Timer;
use hal::uart::{DataBits, StopBits, UartConfig, UartPeripheral};
use hal::watchdog::Watchdog;

use pac::Peripherals;
use pac::CorePeripherals;

use hal::fugit::RateExtU32;

use embedded_hal::spi::{SpiBus, MODE_0};
use rp_pico::hal::pio::PIOExt;

use audio::{AudioOut, AUDIO_OUT};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::DrawTarget;

pub mod sharp_memory_display;
pub use sharp_memory_display::SharpDisplay;

use crate::hardware::uart::{UartHandler, UART_HANDLER};
pub mod audio;
mod ptt;
pub(crate) mod uart;

// Wow, those are some types ---------------------------------------------------

pub type DisplaySpi = hal::spi::Spi<
    hal::spi::Enabled,
    pac::SPI0,
    (
        gpio::Pin<gpio::bank0::Gpio19, gpio::FunctionSpi, gpio::PullDown>,
        gpio::Pin<gpio::bank0::Gpio18, gpio::FunctionSpi, gpio::PullDown>,
    ),
>;

pub type DisplayCS = gpio::Pin<gpio::bank0::Gpio17, gpio::FunctionSio<gpio::SioOutput>, gpio::PullDown>;

pub type GpsUart = UartPeripheral<
    hal::uart::Enabled,
    pac::UART0,
    (
        gpio::Pin<gpio::bank0::Gpio0, gpio::FunctionUart, gpio::PullDown>,
        gpio::Pin<gpio::bank0::Gpio1, gpio::FunctionUart, gpio::PullDown>,
    ),
>;

pub struct Hardware {
    pub display: SharpDisplay<DisplaySpi, DisplayCS>,
    pub timer: Timer
}

impl Hardware {
    pub fn init(mut pac: Peripherals, _core: CorePeripherals) -> Self {
        // Get all the usual objects ready
        let mut watchdog = Watchdog::new(pac.WATCHDOG);
        let clocks = hal::clocks::init_clocks_and_plls(
            rp_pico::XOSC_CRYSTAL_FREQ,
            pac.XOSC,
            pac.CLOCKS,
            pac.PLL_SYS,
            pac.PLL_USB,
            &mut pac.RESETS,
            &mut watchdog,
        ).unwrap();

        let sio = Sio::new(pac.SIO);

        let pins = Pins::new(
            pac.IO_BANK0,
            pac.PADS_BANK0,
            sio.gpio_bank0,
            &mut pac.RESETS,
        );

        // Init the display ----------------------------------------------------

        let disp_mosi = pins.gpio19.into_function();
        let disp_sclk = pins.gpio18.into_function();
        let disp_cs = pins.gpio17.into_push_pull_output();
        let disp_spi = hal::spi::Spi::<_, _, _, 8>::new(
            pac.SPI0,
            (disp_mosi, disp_sclk)
        );
        let mut disp_spi = disp_spi.init(
            &mut pac.RESETS,
            clocks.peripheral_clock.freq(),
            2.MHz(),
            MODE_0,
        );

        // Test the SPI connection and panic if it's bad (why?)
        disp_spi.write(&[0]).unwrap();

        // Instantiate the display driver
        let mut display = SharpDisplay::new(disp_spi, disp_cs);
        display.clear(BinaryColor::Off).unwrap();
        display.flush().unwrap();

        // Init the GPS UART ---------------------------------------------------

        let uart_pins = (
            pins.gpio0.into_function(),
            pins.gpio1.into_function(),
        );
        let uart = UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
            .enable(
                UartConfig::new(38400.Hz(), DataBits::Eight, None, StopBits::One),
                clocks.peripheral_clock.freq(),
            )
            .unwrap();

        // Create the handler
        let uart_handler = UartHandler::new(uart);
        // IMMEDIATELY move UartHandler into the global for the IRQ
        critical_section::with(|cs| {
            UART_HANDLER.borrow(cs).replace(Some(uart_handler));
        });

        // Init the audio system -----------------------------------------------
        let (pio, sm, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
        let dma = pac.DMA.split(&mut pac.RESETS);

        pins.gpio20.into_function::<FunctionPio0>();
        pins.gpio21.into_function::<FunctionPio0>();
        pins.gpio22.into_function::<FunctionPio0>();

        // Init the PTT pin
        let mut ptt = ptt::PttWrapper::new(pins.gpio15);
        ptt.key(false); // De-assert PTT

        let audio = AudioOut::new(dma, pio, sm, ptt, &clocks);
        // IMMEDIATELY move AudioOut into the global for the IRQ
        critical_section::with(|cs| {
            AUDIO_OUT.borrow(cs).replace(Some(audio));
        });

        // Init the timer ------------------------------------------------------

        let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

        // Package everything up -----------------------------------------------

        Self {
            display,
            timer,
        }

    }
}