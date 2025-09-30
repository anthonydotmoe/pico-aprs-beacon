use heapless::Vec;

pub const MAX_DIGIPEATERS: usize = 8;
pub const MAX_INFO_LEN: usize = 256;
pub const MAX_FRAME_LEN: usize = 330;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AddressField {
    raw: [u8; 7],
}

impl AddressField {
    pub fn from_text(call: &str, ssid: u8) -> Result<Self, ()> {
        Ok(Self {
            raw: encode_callsign(call, ssid)?,
        })
    }

    #[allow(dead_code)]
    pub const fn from_raw(raw: [u8; 7]) -> Self {
        Self { raw }
    }

    pub fn raw(&self) -> [u8; 7] {
        self.raw
    }
}

fn encode_callsign(call: &str, ssid: u8) -> Result<[u8; 7], ()> {
    let mut out = [b' ' << 1; 7]; // Pre-fill with shifted spaces

    // Take up to 6 uppercase ASCII characters, pad with spaces if needed
    for (i, c) in call.chars().take(6).enumerate() {
        if !c.is_ascii() {
            return Err(());
        }
        let up = c.to_ascii_uppercase();
        out[i] = (up as u8) << 1;
    }

    // SSID:
    // bits 7-5 = 011
    // bits 4-1 = SSID << 1
    // bit 0 = end-of-address
    let ssid = ssid & 0x0F;
    let ssid_byte = 0b0110_0000 | (ssid << 1);

    out[6] = ssid_byte;
    
    Ok(out)
}

fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;

    for &byte in data {
        for i in 0..8 {
            let bit = (byte >> i) & 0x01;
            let carry = (crc & 0x0001) ^ bit as u16;
            crc >>= 1;
            if carry != 0 {
                crc ^= 0x8408; // Bit-reversed 0x1021
            }
        }
    }

    crc ^ 0xFFFF
}

fn push_address(
    frame: &mut Vec<u8, MAX_FRAME_LEN>,
    address: &AddressField,
    last: bool,
) -> Result<(), ()> {
    let mut bytes = address.raw();
    if last {
        bytes[6] |= 0x01;
    } else {
        bytes[6] &= !0x01;
    }

    frame.extend_from_slice(&bytes).map_err(|_| ())
}

pub fn build_ui_frame(
    dest: AddressField,
    src: AddressField,
    digipeaters: &[AddressField],
    info: &[u8],
) -> Result<Vec<u8, MAX_FRAME_LEN>, ()> {
    if digipeaters.len() > MAX_DIGIPEATERS {
        return Err(());
    }

    if info.len() > MAX_INFO_LEN {
        return Err(());
    }

    let mut frame = Vec::<u8, MAX_FRAME_LEN>::new();

    push_address(&mut frame, &dest, false)?;
    push_address(&mut frame, &src, digipeaters.is_empty())?;
    for (idx, digi) in digipeaters.iter().enumerate() {
        let last = idx == digipeaters.len() - 1;
        push_address(&mut frame, digi, last)?;
    }

    frame.push(0x03).map_err(|_| ())?;  // UI frame
    frame.push(0xF0).map_err(|_| ())?;  // No layer 3 protocol

    frame.extend_from_slice(info).map_err(|_| ())?;

    let crc = crc16(&frame);
    frame.push((crc & 0xFF) as u8).map_err(|_| ())?;
    frame.push((crc >> 8) as u8).map_err(|_| ())?;

    Ok(frame)
}

#[cfg(test)]
mod tests {
    use defmt::expect;

    use crate::aprs::{Coordinate, PositionReport};

    #[test]
    fn position_frame_matches_reference_payload() {
        let latitude = Coordinate {
            microdegrees: 49_058_334,
        };
        let longitude = Coordinate {
            microdegrees: -72_029_167
        };

        let mut comment = heapless::String::<43>::new();
        comment.push_str("PHG0020Test 001234").unwrap();

        let report = PositionReport {
            latitude,
            longitude,
            symbol_table: '/',
            symbol_code: 'b',
            comment: Some(comment),
            timestamp: None,
            messaging: false,
        };

        let frame = crate::aprs::build_position_frame(&report).expect("frame build");

        let expected: [u8; 63] = [
            130, 160, 180, 64, 64, 64, 96, 156, 96, 134, 130, 152, 152, 110, 174, 146, 136, 138, 98,
            64, 99, 3, 240, 33, 52, 57, 48, 51, 46, 53, 48, 78, 47, 48, 55, 50, 48, 49, 46, 55, 53,
            87, 98, 80, 72, 71, 48, 48, 50, 48, 84, 101, 115, 116, 32, 48, 48, 49, 50, 51, 52, 190,
            179,
        ];

        assert_eq!(frame.as_slice(), &expected);
    }

}