use core::iter::repeat_n;

use crate::{
    color::{ColorCorrection, LinearSrgb, RgbChannels},
    driver::clocked::ClockedLed,
    util::component::Component,
};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Lpd8806;

impl Lpd8806 {
    pub const fn frame_buffer_size(pixel_count: usize) -> usize {
        // LPD8806 uses a start frame of 32 zeros and an end frame of at least (n/2) zeros (same as APA102)
        // Each LED uses 3 bytes (G, R, B), each 7-bit data with MSB set
        4 + pixel_count * 3 + (pixel_count - 1).div_ceil(16)
    }
}

impl ClockedLed for Lpd8806 {
    type Word = u8;
    type Color = LinearSrgb;

    fn start() -> impl IntoIterator<Item = Self::Word> {
        [0x00, 0x00, 0x00, 0x00]
    }

    fn led(
        linear_rgb: LinearSrgb,
        _brightness: f32,
        correction: ColorCorrection,
    ) -> impl IntoIterator<Item = Self::Word> {
        // LPD8806 uses 7 bits per color and requires the high bit set to 1
        // Channel order is typically GRB
        let (r, g, b) = (linear_rgb.red, linear_rgb.green, linear_rgb.blue);

        let r = r * correction.red;
        let g = g * correction.green;
        let b = b * correction.blue;

        let (r16, g16, b16) = (
            Component::from_normalized_f32(r),
            Component::from_normalized_f32(g),
            Component::from_normalized_f32(b),
        );

        // Map 16-bit to 7-bit values with rounding, then set MSB
        let to_7bit = |x: u16| -> u8 {
            // map16_to_8 rounding then clamp to 7 bits
            let mut v = if x == 0 {
                0
            } else if x >= 0xff00 {
                0xff
            } else {
                ((x + 128) >> 8) as u8
            };
            v >>= 1; // 7-bit
            0x80 | v
        };

        // LPD8806 expects GRB order on the wire
        let bytes = RgbChannels::GRB.reorder([
            to_7bit(r16),
            to_7bit(g16),
            to_7bit(b16),
        ]);

        [bytes[0], bytes[1], bytes[2]]
    }

    fn end(pixel_count: usize) -> impl IntoIterator<Item = Self::Word> {
        let num_bytes = (pixel_count - 1).div_ceil(16);
        repeat_n(0u8, num_bytes)
    }
}
