use defmt::info;
use embedded_hal::{delay::DelayNs, digital::OutputPin};
use fugit::{MegahertzU32 as Megahertz, NanosDurationU32 as Nanoseconds};
use palette::{num::Round, IntoColor, LinSrgb, Srgb};

pub trait LedDriver {
    type Error;
    type Color;

    fn write<C, const N: usize>(&mut self, pixels: [C; N]) -> Result<(), Self::Error>
    where
        C: IntoColor<Self::Color>;
}

#[derive(Debug)]
pub enum RgbOrder {
    RGB,
    RBG,
    GRB,
    GBR,
    BRG,
    BGR,
}

pub trait ClockedWriter {
    type Word: Copy + 'static;
    type Error;

    fn write(&mut self, buf: &[Self::Word]) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub struct ClockedWriterBitBang<Data: OutputPin, Clock: OutputPin, Delay: DelayNs> {
    data: Data,
    clock: Clock,
    delay: Delay,
    t_half_cycle_ns: u32,
}

impl<Data: OutputPin, Clock: OutputPin, Delay: DelayNs> ClockedWriterBitBang<Data, Clock, Delay> {
    pub fn new(data: Data, clock: Clock, delay: Delay, data_rate: Megahertz) -> Self {
        let t_cycle: Nanoseconds = data_rate.into_duration();
        let t_half_cycle = t_cycle / 2;
        let t_half_cycle_ns = t_half_cycle.to_nanos();

        Self {
            data,
            clock,
            delay,
            t_half_cycle_ns,
        }
    }
}

#[derive(Debug)]
pub enum ClockedWriterBitBangError<Data: OutputPin, Clock: OutputPin> {
    Data(Data::Error),
    Clock(Clock::Error),
}

impl<Data: OutputPin, Clock: OutputPin, Delay: DelayNs> ClockedWriter
    for ClockedWriterBitBang<Data, Clock, Delay>
{
    type Error = ClockedWriterBitBangError<Data, Clock>;
    type Word = u8;

    fn write(&mut self, buffer: &[Self::Word]) -> Result<(), Self::Error> {
        info!("write: {}", buffer);

        // For each byte in the buffer, iterate over bit masks in descending order.
        for byte in buffer {
            for bit_position in [128, 64, 32, 16, 8, 4, 2, 1] {
                match byte & bit_position {
                    0 => self.data.set_low(),
                    _ => self.data.set_high(),
                }
                .map_err(ClockedWriterBitBangError::Data)?;

                self.delay.delay_ns(self.t_half_cycle_ns);

                self.clock
                    .set_high()
                    .map_err(ClockedWriterBitBangError::Clock)?;

                self.delay.delay_ns(self.t_half_cycle_ns);

                self.clock
                    .set_low()
                    .map_err(ClockedWriterBitBangError::Clock)?;
            }
        }
        Ok(())
    }
}

// Examples
// - WS2801:
// - Apa102:
//   - https://hackaday.com/2014/12/09/digging-into-the-apa102-serial-led-protocol/
//   - https://www.pololu.com/product/2554

#[derive(Debug)]
pub struct Apa102<Writer: ClockedWriter> {
    writer: Writer,
    brightness: f32,
    rgb_order: RgbOrder,
}

impl<Writer: ClockedWriter> Apa102<Writer> {
    pub fn new(writer: Writer, brightness: f32) -> Self {
        Self {
            writer,
            brightness,
            rgb_order: RgbOrder::BGR,
        }
    }
    pub fn new_with_rgb_order(writer: Writer, brightness: f32, rgb_order: RgbOrder) -> Self {
        Self {
            writer,
            brightness,
            rgb_order,
        }
    }
}

impl<Writer> LedDriver for Apa102<Writer>
where
    Writer: ClockedWriter<Word = u8>,
{
    type Error = <Writer as ClockedWriter>::Error;
    type Color = Srgb;

    fn write<Color, const N: usize>(&mut self, pixels: [Color; N]) -> Result<(), Self::Error>
    where
        Color: IntoColor<Self::Color>,
    {
        self.writer.write(&[0x00, 0x00, 0x00, 0x00])?;

        // TODO handle brightness how APA102HD works in FastLED

        let brightness = 0b11100000 | (map_f32_to_u8_range(self.brightness, 31) & 0b00011111);

        for item in pixels.into_iter() {
            let item: Srgb = item.into_color();
            let item: LinSrgb = item.into_color();
            let item: LinSrgb<u8> = item.into_format();
            let led_frame = match self.rgb_order {
                RgbOrder::RGB => [brightness, item.red, item.green, item.blue],
                RgbOrder::RBG => [brightness, item.red, item.blue, item.green],
                RgbOrder::GRB => [brightness, item.green, item.red, item.blue],
                RgbOrder::GBR => [brightness, item.green, item.blue, item.red],
                RgbOrder::BRG => [brightness, item.blue, item.red, item.green],
                RgbOrder::BGR => [brightness, item.blue, item.green, item.red],
            };
            self.writer.write(&led_frame)?;
        }

        let end_frame_length = (N - 1).div_ceil(16);
        for _ in 0..end_frame_length {
            self.writer.write(&[0x00])?
        }

        Ok(())
    }
}

fn map_f32_to_u8_range(value: f32, max: u8) -> u8 {
    let clamped = value.clamp(0.0, 1.0);
    (clamped * (max as f32)).round() as u8
}

// Examples
// - WS2812B: https://cdn-shop.adafruit.com/datasheets/WS2812B.pdf

pub trait LedClockless {
    const T_0H: Nanoseconds;
    const T_0L: Nanoseconds;
    const T_1H: Nanoseconds;
    const T_1L: Nanoseconds;
    const T_RESET: Nanoseconds;

    const OUTPUT_COUNT: usize;

    fn t_cycle() -> Nanoseconds {
        (Self::T_0H + Self::T_0L).max(Self::T_1H + Self::T_1L)
    }
}
