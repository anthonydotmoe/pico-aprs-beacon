use crate::app::Shared;
use crate::aprs::{self, Coordinate};
use crate::ax25::{self, TxBits};
use crate::sched::Tickable;

pub struct BeaconTask {
    next_tx_time: u64,
}

impl BeaconTask {
    pub fn new() -> Self {
        Self {
            next_tx_time: 0,
        }
    }

    fn run(&mut self, now: u64, shared: &mut Shared) {

        if shared.nmea.fix_type.is_none() {
            // No fix, try again later.
            self.next_tx_time = now + 5_000; // 5 sec.
            return;
        }

        // Only proceed when all info is present.
        let (lat, lon) = match (shared.nmea.latitude(), shared.nmea.longitude()) {
            (Some(lat), Some(lon)) => (lat, lon),
            _ => {
                // We have a fix, but not everything else; try shortly
                self.next_tx_time = now + 1_000;
                return;
            }
        };

        // Prepare the packet
        shared.pos_rpt.latitude = Coordinate::from_float(lat);
        shared.pos_rpt.longitude = Coordinate::from_float(lon);

        // If modem hasn't consumed the previous frames, reschedule
        if shared.txq.is_full() {
            self.next_tx_time = now + 1_000;
            return;
        }

        // Build bytes -> stuffed bits as Bitstream
        // Encode the packet as bytes
        let packet = aprs::build_position_frame(&shared.pos_rpt).expect("build frame");
        let bits: TxBits = ax25::build_on_air(packet).expect("build bitstream");
        
        // Send it off to the modem
        shared.txq.push_back(bits).ok();

        // Schedule the next beacon
        self.next_tx_time = now + (30 * 60 * 1_000); // 30 min
    }
}

impl Tickable for BeaconTask {
    fn next_run_at(&self) -> u64 {
        self.next_tx_time
    }

    fn tick(&mut self, now: u64, shared: &mut Shared) {
        self.run(now, shared);
    }
}