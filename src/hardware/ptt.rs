use rp_pico::hal;
use hal::gpio::{self, InOutPin, Pin};
use embedded_hal::digital::OutputPin;

type PttPin = Pin<gpio::bank0::Gpio15, gpio::FunctionNull, gpio::PullDown>;

pub struct PttWrapper {
    pin: InOutPin<PttPin>,
}

impl PttWrapper {
    pub fn new(inner: PttPin) -> PttWrapper
    {
        let iopin = InOutPin::new(inner);
        Self {
            pin: iopin
        }
    }

    pub fn key(&mut self, on: bool) {
        // Safety: Setting pin states is infallable
        let _ = if on {
            self.pin.set_low()
        } else {
            self.pin.set_high()
        };
    }

    // I never call this
    /*
    pub fn release(self) -> PttPin {
        self.pin.release()
    }
    */
}