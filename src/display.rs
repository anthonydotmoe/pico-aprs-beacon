use crate::app::Shared;
use crate::hardware::{SharpDisplay, DisplaySpi, DisplayCS};
use crate::sched::Tickable;

use embedded_graphics::prelude::*;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::text::Text;

use core::fmt::Write;

pub struct DisplayTask {
    display: SharpDisplay<DisplaySpi, DisplayCS>,

    next_run_at: u64,
}

const fn fps_to_ms(fps: u64) -> u64 {
    ((1.0 / fps as f32) * 1000.0) as u64
}

impl DisplayTask {
    pub fn new(display: SharpDisplay<DisplaySpi, DisplayCS>) -> Self {
        Self {
            display,
            next_run_at: 0,
        }
    }

    pub fn run(&mut self, now: u64, shared: &mut Shared) {

        let mut text = heapless::String::<128>::new();
        write!(&mut text, "Hello, display!\n{}\n{}\ndelta: {}", now, self.next_run_at, now - self.next_run_at).unwrap();

        self.next_run_at = now + fps_to_ms(30);

        self.display.clear(BinaryColor::Off).unwrap();
        let style = MonoTextStyle::new(&FONT_10X20, BinaryColor::On);
        Text::new(&text, Point::new(16, 16), style)
            .draw(&mut self.display)
            .unwrap();

        
        // Draw fix information
        text.clear();
        let _ = write!(&mut text, "Fix: {:?}\nLat: {:.4?}\nLon: {:.4?}\nTim: {:?}\nHed: {:?}\nSpd: {:?}",
            shared.nmea.fix_type,
            shared.nmea.latitude,
            shared.nmea.longitude,
            shared.nmea.fix_time,
            shared.nmea.true_course,
            shared.nmea.speed_over_ground,
        );
        let style = MonoTextStyle::new(&FONT_10X20, BinaryColor::On);
        Text::new(&text, Point::new(8, 128), style)
            .draw(&mut self.display)
            .unwrap();

        self.display.flush().unwrap();
    }
}

impl Tickable for DisplayTask {
    fn next_run_at(&self) -> u64 {
        self.next_run_at
    }
    fn tick(&mut self, now: u64, shared: &mut Shared) {
        self.run(now, shared);
    }
}