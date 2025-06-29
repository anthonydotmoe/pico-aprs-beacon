use nmea::{Nmea, SentenceType};
use rp_pico as bsp;
use bsp::hal;
use hal::pac;

use pac::{CorePeripherals, Peripherals};

use crate::gps::GpsTask;
use crate::hardware::Hardware;
use crate::display::DisplayTask;
use crate::sched::{Scheduler, Tickable};

pub struct Shared {
    pub nmea: Nmea,
}

impl Shared {
    fn new() -> Self {
        let nmea = Nmea::create_for_navigation(&[
            SentenceType::GGA,
            SentenceType::GLL,
            SentenceType::GSA,
            SentenceType::GSV,
            SentenceType::GNS,
            SentenceType::RMC,
            SentenceType::VTG,
        ]).unwrap();
        Self {
            nmea
        }
    }
}

pub fn run(pac: Peripherals, core: CorePeripherals) -> ! {
    let hw = Hardware::init(pac, core);
    let mut display_task = DisplayTask::new(hw.display);
    let mut gps_task = GpsTask::new();

    let mut task_list: [&mut dyn Tickable; 2] = [
        &mut display_task,
        &mut gps_task,
    ];

    let mut shared = Shared::new();
    
    let mut scheduler = Scheduler::new(&mut task_list, &mut shared);

    loop {
        let now = hw.timer.get_counter().ticks() / 1_000; // ms
        scheduler.run(now);
        //cortex_m::asm::wfi();
    }
}