use core::fmt::Write;

use rp_pico::hal::rom_data::double_funcs::{dmul, double_to_int};

/// High-level APRS info field representation
pub enum AprsInfo {
    Position(PositionReport),
    Unknown(u8, heapless::Vec<u8, 256>),
}

#[derive(Debug)]
pub enum Timestamp {
    Dhm { day: u8, hour: u8, minute: u8 },
    Hms { hour: u8, minute: u8, second: u8 },
    LocalTime { hour: u8, minute: u8 },
}

impl Timestamp {
    pub fn encode<W: Write>(&self, out: &mut W) -> Result<(), core::fmt::Error> {
        match self {
            Self::Dhm { day, hour, minute } => {
                write!(out, "{:02}{:02}{:02}z", day, hour, minute)
            },
            Self::Hms { hour, minute, second } => {
                write!(out, "{:02}{:02}{:02}h", hour, minute, second)
            },
            Self::LocalTime { hour, minute } => {
                write!(out, "{:02}{:02}/", hour, minute)
            },
        }
    }
}

#[derive(Debug)]
pub struct Coordinate {
    pub microdegrees: i32,
}

impl Coordinate {
    pub fn from_float(d: f64) -> Coordinate {
        let microdegrees = double_to_int(dmul(d, 1_000_000.0));
        Coordinate {
            microdegrees,
        }
    }

    pub fn to_aprs<W: Write>(&self, lat: bool, out: &mut W) -> Result<(), core::fmt::Error> {
        let raw = self.microdegrees.abs();
        let deg = raw / 1_000_000;
        let frac = raw % 1_000_000;
        let minutes = frac * 60 / 1_000_000;
        let hundredths = ((frac * 60_00 / 1_000_000) % 100) as u8;

        if lat {
            write!(out, "{:02}{:02}.{:02}", deg, minutes, hundredths)?;
        } else {
            write!(out, "{:03}{:02}.{:02}", deg, minutes, hundredths)?;
        }

        let suffix = match (lat, self.microdegrees >= 0) {
            (true, true) => 'N',
            (true, false) => 'S',
            (false, true) => 'E',
            (false, false) => 'W',
        };
        out.write_char(suffix)
    }
}

#[derive(Debug)]
pub struct PositionReport {
    pub latitude: Coordinate,
    pub longitude: Coordinate,
    pub symbol_table: char,
    pub symbol_code: char,
    pub comment: Option<heapless::String<43>>,
    pub timestamp: Option<Timestamp>,
    pub messaging: bool,
}

impl PositionReport {
    pub fn encode<W: Write>(&self, buf: &mut W) -> Result<(), ()> {
        // Data format identifier
        let symbol_prefix = match (&self.timestamp, self.messaging) {
            (Some(_), true) => '@',
            (Some(_), false) => '/',
            (None, true) => '=',
            (None, false) => '!',
        };
        buf.write_char(symbol_prefix).unwrap();

        // Timestamp
        if let Some(ts) = &self.timestamp {
            ts.encode(buf).unwrap();
        }

        // Latitude
        self.latitude.to_aprs(true, buf).unwrap();

        // Sym Table ID
        buf.write_char(self.symbol_table).unwrap();

        // Longitude
        self.longitude.to_aprs(false, buf).unwrap();

        // Symbol Code
        buf.write_char(self.symbol_code).unwrap();

        // Comment
        if let Some(s) = &self.comment {
            buf.write_str(s).unwrap();
        }

        Ok(())
    }
}

fn split_callsign_ssid(input: &str) -> (&str, u8) {
    let (call, ssid) = match input.split_once('-') {
        Some((call, ssid)) => (call.trim(), ssid),
        None => (input.trim(), "0"),
    };

    let ssid = ssid.parse::<u8>().unwrap_or(0);
    (call, ssid)
}

pub fn build_position_frame(
    report: &PositionReport,
) -> Result<heapless::Vec<u8, {crate::ax25::MAX_FRAME_LEN}>, ()> {
    use crate::ax25::{self, AddressField};

    let (dest_call, dest_ssid) = split_callsign_ssid(crate::co::TOCALL);
    let (src_call, src_ssid) = split_callsign_ssid(crate::co::MYCALL);

    let dest = AddressField::from_text(dest_call, dest_ssid)?;
    let src = AddressField::from_text(src_call, src_ssid)?;
    let digipeaters = [AddressField::from_text("WIDE1", 1)?];

    let mut info = heapless::String::<{ crate::ax25::MAX_INFO_LEN }>::new();
    report.encode(&mut info)?;

    ax25::build_ui_frame(dest, src, &digipeaters, info.as_bytes())
}