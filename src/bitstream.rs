/*
pub struct Bitstream {
    data: Vec<u8>,
    bit_pos: u8,
    curr_byte: u8,
}

impl Bitstream {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            bit_pos: 0,
            curr_byte: 0,
        }
    }

    pub fn push_bit(&mut self, bit: bool) {
        if bit {
            self.curr_byte |= 1 << self.bit_pos;
        }
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.data.push(self.curr_byte);
            self.curr_byte = 0;
            self.bit_pos = 0;
        }
    }

    pub fn finish(mut self) -> Vec<u8> {
        if self.bit_pos != 0 {
            self.data.push(self.curr_byte);
        }
        self.data
    }
}
*/