use core::cell::RefCell;
use core::convert::Infallible;
use core::fmt::{self, Write};
use defmt_rtt as _;
use display_interface::DisplayError;
use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_rp::peripherals::SPI1;
use embassy_rp::spi::{ClkPin, Config as SpiConfig, MosiPin, Spi};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use embedded_graphics::Pixel;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_hal_bus::spi::ExclusiveDevice;
use log::{error, info};
use ssd1306::Ssd1306;
use ssd1306::command::Command;
use ssd1306::prelude::*;
use ssd1306::size::DisplaySize as DisplaySizeTrait;
use static_cell::StaticCell;

#[derive(Debug, Copy, Clone)]
pub struct DisplaySize132x64;
impl DisplaySizeTrait for DisplaySize132x64 {
    const WIDTH: u8 = 132;
    const HEIGHT: u8 = 64;
    type Buffer = [u8; ((64 * 132) / 8) as usize];

    fn configure(&self, iface: &mut impl WriteOnlyDataCommand) -> Result<(), DisplayError> {
        Command::ComPinConfig(true, false).send(iface)
    }
}

// Display
type PinSpi = SPI1;
type DisplaySize = DisplaySize132x64;
type DisplayType = Ssd1306<
    SPIInterface<
        ExclusiveDevice<
            Spi<'static, PinSpi, embassy_rp::spi::Blocking>,
            Output<'static>,
            embedded_hal_bus::spi::NoDelay,
        >,
        Output<'static>,
    >,
    DisplaySize,
    ssd1306::mode::BufferedGraphicsMode<DisplaySize>,
>;

static DISPLAY: StaticCell<DisplayType> = StaticCell::new();
/// Where the visible window starts in the 132x64 controller buffer.
const VISIBLE_OFFSET: (u32, u32) = (4, 32);
const ROW_BYTES: usize = 16;
const ROWS: usize = 32;

static NEXT_FRAME: Signal<CriticalSectionRawMutex, DisplayProxy> = Signal::new();
static SHOWN_FRAME: Mutex<CriticalSectionRawMutex, RefCell<DisplayProxy>> =
    Mutex::new(RefCell::new(DisplayProxy::new()));

pub async fn init(
    spawner: &Spawner,
    pin_spi: Peri<'static, PinSpi>,
    pin_clk: Peri<'static, impl ClkPin<PinSpi>>,
    pin_mosi: Peri<'static, impl MosiPin<PinSpi>>,
    pin_dc: Peri<'static, impl Pin>,
    pin_rst: Peri<'static, impl Pin>,
    pin_cs: Peri<'static, impl Pin>,
) {
    // Init SPI
    let mut spi_config = SpiConfig::default();
    spi_config.frequency = 8_000_000; // 8 MHz

    let spi = Spi::new_blocking_txonly(pin_spi, pin_clk, pin_mosi, spi_config);

    // Configure control pins
    let dc_pin = Output::new(pin_dc, Level::Low);
    let mut rst_pin = Output::new(pin_rst, Level::High);
    let cs_pin = Output::new(pin_cs, Level::High);

    // Reset the display
    rst_pin.set_low();
    Timer::after_millis(10).await;
    rst_pin.set_high();
    Timer::after_millis(10).await;

    // Create the display interface
    let spi_device = ExclusiveDevice::new_no_delay(spi, cs_pin).unwrap();
    let interface = SPIInterface::new(spi_device, dc_pin);

    info!(
        "Creating display driver ({}x{})...",
        DisplaySize::WIDTH,
        DisplaySize::HEIGHT
    );
    let display = DISPLAY
        .init(Ssd1306::new(interface, DisplaySize132x64, DisplayRotation::Rotate0).into_buffered_graphics_mode());

    match display.init() {
        Ok(()) => {
            spawner.spawn(display_task(display).unwrap());

            info!("Display initialized successfully!");
        }
        Err(_) => {
            error!("Display initialization failed!");
        }
    }
}

#[embassy_executor::task]
async fn display_task(display: &'static mut DisplayType) {
    // Start from a known blank panel, so later frames only need to send what changed.
    display.clear_buffer();

    if let Err(error) = display.flush() {
        error!("Display error: {error:?}");
    }

    let mut shown = DisplayProxy::new();

    loop {
        let frame = NEXT_FRAME.wait().await;

        frame
            .changes(&shown)
            .for_each(|(x, y, lit)| display.set_pixel(x + VISIBLE_OFFSET.0, y + VISIBLE_OFFSET.1, lit));

        // The driver sends only the rectangle around the pixels that changed.
        if let Err(error) = display.flush() {
            error!("Display error: {error:?}");
        }

        SHOWN_FRAME.lock(|cell| cell.replace(frame.clone()));
        shown = frame;
    }
}

/// A frame of the visible 128x32 window that modes draw into; (0, 0) is the top-left pixel the panel
/// shows. [`DisplayProxy::flush`] publishes it, and the display shows the latest published frame.
#[derive(Clone)]
pub struct DisplayProxy([[u8; ROW_BYTES]; ROWS]);

impl DisplayProxy {
    pub const WIDTH: i32 = 128;
    pub const HEIGHT: i32 = 32;

    pub const fn new() -> Self {
        Self([[0; ROW_BYTES]; ROWS])
    }

    pub fn flush(&self) {
        NEXT_FRAME.signal(self.clone());
    }

    /// The frame the display shows now.
    pub fn shown() -> Self {
        SHOWN_FRAME.lock(|cell| cell.borrow().clone())
    }

    fn index(point: Point) -> Option<(usize, usize, u8)> {
        let x = usize::try_from(point.x).ok().filter(|&x| x < ROW_BYTES * 8)?;
        let y = usize::try_from(point.y).ok().filter(|&y| y < ROWS)?;

        Some((y, x / 8, 0x80 >> (x % 8)))
    }

    pub fn invert(&mut self, area: &Rectangle) {
        area.points()
            .filter_map(Self::index)
            .for_each(|(row, byte, mask)| self.0[row][byte] ^= mask);
    }

    /// Pixels that differ from `previous`, as `(x, y, lit)`.
    fn changes<'a>(&'a self, previous: &'a Self) -> impl Iterator<Item = (u32, u32, bool)> + 'a {
        self.0
            .iter()
            .flatten()
            .zip(previous.0.iter().flatten())
            .zip(0u32..)
            .filter(|((now, before), _)| now != before)
            .flat_map(|((&now, &before), index)| {
                (0..8u32)
                    .filter(move |bit| (now ^ before) & (0x80 >> bit) != 0)
                    .map(move |bit| (index % 16 * 8 + bit, index / 16, now & (0x80 >> bit) != 0))
            })
    }

    /// The frame as hex, one row of 16 bytes after another, most significant bit leftmost.
    pub fn write_hex(&self, out: &mut impl Write) -> fmt::Result {
        self.0.iter().flatten().try_for_each(|byte| write!(out, "{byte:02x}"))
    }
}

impl OriginDimensions for DisplayProxy {
    fn size(&self) -> Size {
        Size::new(Self::WIDTH.unsigned_abs(), Self::HEIGHT.unsigned_abs())
    }
}

impl DrawTarget for DisplayProxy {
    type Color = BinaryColor;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        pixels
            .into_iter()
            .filter_map(|Pixel(point, color)| Self::index(point).map(|index| (index, color)))
            .for_each(|((row, byte, mask), color)| match color {
                BinaryColor::On => self.0[row][byte] |= mask,
                BinaryColor::Off => self.0[row][byte] &= !mask,
            });

        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let fill = if color.is_on() { 0xFF } else { 0x00 };

        self.0 = [[fill; ROW_BYTES]; ROWS];

        Ok(())
    }
}
