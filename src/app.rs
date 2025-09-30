use core::str::FromStr;

use nmea::{Nmea, SentenceType};
use rp_pico as bsp;
use bsp::hal;
use hal::pac;

use pac::{CorePeripherals, Peripherals};

use crate::aprs::{Coordinate, PositionReport};
use crate::ax25::TxBits;
use crate::beacon::BeaconTask;
use crate::display::DisplayTask;
use crate::gps::GpsTask;
use crate::hardware::Hardware;
use crate::modem::AfskModulator;
use crate::sched::{Scheduler, Tickable};

pub struct Shared {
    pub nmea: Nmea,
    pub pos_rpt: PositionReport,
    pub txq: heapless::Deque<TxBits, 2>,
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

        let pos_rpt = PositionReport {
            latitude: Coordinate { microdegrees: 0 },
            longitude: Coordinate { microdegrees: 0 },
            symbol_table: '/',
            symbol_code: 'n',
            comment: Some(heapless::String::<43>::from_str("github.com/anthonydotmoe/pico-aprs-beacon").unwrap()),
            timestamp: None,
            messaging: false,
        };

        Self {
            nmea,
            pos_rpt,
            txq: heapless::Deque::new(),
        }
    }
}

pub fn run(pac: Peripherals, core: CorePeripherals) -> ! {
    let hw = Hardware::init(pac, core);
    let mut display_task = DisplayTask::new(hw.display);
    let mut gps_task = GpsTask::new();
    let mut beacon_task = BeaconTask::new();
    let mut modem_task = AfskModulator::new();

    let mut task_list: [&mut dyn Tickable; 4] = [
        &mut display_task,
        &mut gps_task,
        &mut beacon_task,
        &mut modem_task,
    ];

    let mut shared = Shared::new();
    
    let mut scheduler = Scheduler::new(&mut task_list, &mut shared);

    loop {
        let now = hw.timer.get_counter().ticks() / 1_000; // ms
        scheduler.run(now);
        //cortex_m::asm::wfi();
    }
}