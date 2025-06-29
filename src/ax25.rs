fn encode_callsign(call: &str, ssid: u8, last: bool) -> [u8; 7] {
    let mut out = [b' ' << 1; 7]; // Pre-fill with shifted spaces

    // Take up to 6 uppercase ASCII characters, pad with spaces if needed
    for (i, c) in call.chars().take(6).enumerate() {
        let up = c.to_ascii_uppercase();
        out[i] = (up as u8) << 1;
    }

    // SSID:
    // bits 7-5 = 011
    // bits 4-1 = SSID << 1
    // bit 0 = end-of-address
    let mut ssid_byte = 0b0110_0000 | ((ssid & 0x0F) << 1);
    if last {
        ssid_byte |= 0b0000_0001;
    }
    out[6] = ssid_byte;

    out
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


/*
#[test]
fn test_crc_known_vector() {
    let testvector: &[u8] = &[
        0x82, 0xA0, 0xB4, 0x60, 0x60, 0x60, 0xE0, // "APZ___"
        0x9C, 0x60, 0x86, 0x82, 0x98, 0x98, 0xE3, // "N0CALL-1"
        0x03, 0xF0, 0x2C, 0x41                    // Control PID ",A"
    ];

    let crc = crc16_x25(testvector);
    assert_eq!(crc, 0x4A76)
}
*/