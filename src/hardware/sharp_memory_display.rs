use core::array;

use embedded_hal::digital::OutputPin;
use embedded_hal::spi::SpiBus;

pub const BLACK: u8 = 0;
pub const WHITE: u8 = 1;
pub const SHARP_BIT_WRITECMD: u8 = reverse_byte(0x01);
pub const SHARP_BIT_VCOM: u8 = reverse_byte(0x02);

const WIDTH: usize = 400;
const HEIGHT: usize = 240;

// One line = line number (1 byte) + pixel data + trailing 0 byte
const LINE_PACKET_LEN: usize = (WIDTH / 8) + 2;
type LineBuffer = [u8; LINE_PACKET_LEN];

static REVERSE_BYTE_TABLE: [u8; 256] = {
    const fn reverse_bits(mut b: u8) -> u8 {
        let mut r = 0;
        let mut i = 0;
        while i < 8 {
            r <<= 1;
            r |= b & 1;
            b >>= 1;
            i += 1;
        }
        r
    }

    let mut table = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = reverse_bits(i as u8);
        i += 1;
    }
    table
};

const fn reverse_byte(b: u8) -> u8 {
    REVERSE_BYTE_TABLE[b as usize]
}

pub struct SharpDisplay<SPI, CS> {
    spi: SPI,
    cs: CS,
    vcom: u8,
    buffer: [LineBuffer; HEIGHT],
    dirty_lines: [bool; HEIGHT],
}

impl<SPI, CS> SharpDisplay<SPI, CS>
where
    SPI: SpiBus,
    CS: OutputPin,
{
    pub fn new(spi: SPI, cs: CS) -> Self {
        let buffer = array::from_fn(|i| {
            let mut line = [0xFF; LINE_PACKET_LEN]; // Default to white (0xFF)
            line[0] = reverse_byte((i + 1) as u8);
            line[LINE_PACKET_LEN - 1] = 0x00;
            line
        });

        Self {
            spi,
            cs,
            vcom: SHARP_BIT_VCOM,
            buffer,
            dirty_lines: [true; HEIGHT],
        }
    }

    pub fn draw_pixel(&mut self, x: usize, y: usize, color: u8) {
        if x >= WIDTH || y >= HEIGHT {
            return;
        }

        let x = WIDTH - 1 - x;
        let y = HEIGHT - 1 - y;

        let byte_idx = x / 8;
        let bit_mask = 1 << (x & 7);
        let byte = &mut self.buffer[y][1 + byte_idx];
        let old = *byte;

        match color {
            BLACK => *byte &= !reverse_byte(bit_mask),
            WHITE => *byte |=  reverse_byte(bit_mask),
            _ => {},
        }

        if *byte != old {
            self.dirty_lines[y] = true;
        }
    }

    /*
    pub fn clear(& mut self) {
        self.buffer.fill(0xFF);
    }
    */

    pub fn flush(&mut self) -> Result<(), DisplayError<CS::Error, SPI::Error>> {
        // Write VCOM
        self.cs.set_high().map_err(DisplayError::pin)?;
        self.spi.write(&[self.vcom | SHARP_BIT_WRITECMD]).map_err(DisplayError::spi)?;

        for (line, dirty) in self.dirty_lines.iter_mut().enumerate() {
            if *dirty {
                self.spi.write(&self.buffer[line]).map_err(DisplayError::spi)?;
                *dirty = false;
            }
        }

        self.spi.write(&[0x00]).map_err(DisplayError::spi)?;
        // Wait for SPI operation to finish before de-asserting CS
        self.spi.flush().map_err(DisplayError::spi)?;
        self.cs.set_low().map_err(DisplayError::pin)?;

        // Toggle VCOM bit
        self.vcom ^= SHARP_BIT_VCOM;

        Ok(())
    }
}

#[derive(Debug)]
pub enum DisplayError<PinE, SpiE> {
    Pin(PinE),
    Spi(SpiE),
}

impl<PinE, SpiE> DisplayError<PinE, SpiE> {
    #[inline]
    fn pin(e: PinE) -> Self { DisplayError::Pin(e) }

    #[inline]
    fn spi(e: SpiE) -> Self { DisplayError::Spi(e) }
}

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::{DrawTarget, OriginDimensions, Size};
use embedded_graphics::Pixel;

impl<SPI, CS> DrawTarget for SharpDisplay<SPI, CS>
where
    SPI: SpiBus,
    CS: OutputPin,
{
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>
    {
        for Pixel(coord, color) in pixels {
            let x = coord.x as usize;
            let y = coord.y as usize;
            let c = match color {
                BinaryColor::On => BLACK,
                BinaryColor::Off => WHITE,
            };
            self.draw_pixel(x, y, c);
        }

        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let fill = match color {
            BinaryColor::On => 0x00,
            BinaryColor::Off => 0xFF,
        };
        for line in self.buffer.iter_mut() {
            line[1..LINE_PACKET_LEN-2].fill(fill)
        }
        self.dirty_lines.fill(true);
        Ok(())
    }
}

impl<SPI, CS> OriginDimensions for SharpDisplay<SPI, CS> {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}