use heapless::Vec;

pub struct Bitstream<const N: usize> {
    buf: Vec<u8, N>,
    curr: u8,
    w_bit_pos: u8,
    // read state
    r_byte_idx: usize,
    r_mask: u8,
    len_bits: usize,
}

impl<const N: usize> Bitstream<N> {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            curr: 0,
            w_bit_pos: 0,
            r_byte_idx: 0,
            r_mask: 1,
            len_bits: 0,
        }
    }

    #[inline]
    pub fn capacity_bits(&self) -> usize { N * 8 }

    #[inline]
    pub fn len_bits(&self) -> usize { self.len_bits }

    /// Push one bit (LSB-first packing). Returns Err if out of space.
    pub fn push_bit(&mut self, bit: bool) -> Result<(), ()> {
        if bit {
            self.curr |= 1 << self.w_bit_pos;
        }
        self.w_bit_pos += 1;
        self.len_bits += 1;

        if self.w_bit_pos == 8 {
            self.w_bit_pos = 0;
            self.buf.push(self.curr).map_err(|_| ())?;
            self.curr = 0;
        }
        Ok(())
    }

    /// Finalize write side (flush partial byte)
    pub fn finish(&mut self) -> Result<(), ()> {
        if self.w_bit_pos != 0 {
            self.buf.push(self.curr).map_err(|_| ())?;
            self.curr = 0;
            self.w_bit_pos = 0;
        }
        Ok(())
    }

    /// Pull one bit out (LSB-first). Returns `None` at end.
    pub fn pull_bit(&mut self) -> Option<bool> {
        let total_bytes = self.buf.len();
        if self.r_byte_idx >= total_bytes {
            return None;
        }
        let bit = (self.buf[self.r_byte_idx] & self.r_mask) != 0;

        // advance mask/index
        self.r_mask <<= 1;
        if self.r_mask == 0 {
            self.r_mask = 1;
            self.r_byte_idx += 1;
        }
        Some(bit)
    }

    /// Access packed bytes and the exact number of valid bits.
    pub fn as_bytes(&self) -> (&[u8], usize) {
        (self.buf.as_slice(), self.len_bits)
    }
}