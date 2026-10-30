use defmt_rtt as _;
use display_interface::DisplayError;
use embassy_executor::Spawner;
use embassy_futures::yield_now;
use embassy_rp::Peri;
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_rp::peripherals::SPI1;
use embassy_rp::spi::{ClkPin, Config as SpiConfig, MosiPin, Spi};
use embassy_sync::blocking_mutex::raw::{RawMutex, ThreadModeRawMutex};
use embassy_sync::channel::Channel;
use embassy_sync::channel::{Sender, TrySendError};
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

const DRAW_BUFFER_SIZE: usize = 128;
#[derive(Debug)]
pub enum DisplayAction<C = BinaryColor>
where
    C: PixelColor,
{
    Clear(C),
    FillSolid(Rectangle, C),
    Draw(heapless::Vec<Pixel<C>, DRAW_BUFFER_SIZE>),
    Flush,
}

static DISPLAY: StaticCell<DisplayType> = StaticCell::new();
/// The panel only shows this window of the 132x64 controller buffer.
const VISIBLE_AREA: Rectangle = Rectangle::new(
    Point::new(4, 32),
    Size::new(DisplayProxy::WIDTH.unsigned_abs(), DisplayProxy::HEIGHT.unsigned_abs()),
);
pub static DISPLAY_CHANNEL: Channel<ThreadModeRawMutex, DisplayAction, 64> = Channel::new();

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
    info!("Display rendering task started");

    let receiver = DISPLAY_CHANNEL.receiver();

    loop {
        let action = receiver.receive().await;

        #[cfg(feature = "debug-link")]
        crate::debug_link::mirror(&action);

        let err = match action {
            DisplayAction::Clear(color) => (*display).clear(color),
            DisplayAction::FillSolid(rect, color) => (*display).fill_solid(&rect, color),
            DisplayAction::Draw(pixels) => (*display).draw_iter(pixels),
            DisplayAction::Flush => (*display).flush(),
        };

        if let Err(e) = err {
            error!("Display error: {e:?}");
        }
    }
}

/// Draws in visible-window coordinates: (0, 0) is the top-left pixel the panel shows, and anything
/// outside [`DisplayProxy::WIDTH`] x [`DisplayProxy::HEIGHT`] is clipped.
pub struct DisplayProxy<'u, T = ThreadModeRawMutex, C = BinaryColor, const CN: usize = 64>
where
    T: RawMutex,
    C: PixelColor,
{
    channel: Sender<'u, T, DisplayAction<C>, CN>,
    /// Pixels from consecutive draws share chunks, so a frame of text costs a few channel slots
    /// instead of one per glyph.
    pending: heapless::Vec<Pixel<C>, DRAW_BUFFER_SIZE>,
}

impl DisplayProxy<'_> {
    pub const WIDTH: i32 = 128;
    pub const HEIGHT: i32 = 32;

    pub fn new() -> Self {
        Self {
            channel: DISPLAY_CHANNEL.sender(),
            pending: heapless::Vec::new(),
        }
    }
}

impl<T, C, const CN: usize> DisplayProxy<'_, T, C, CN>
where
    T: RawMutex,
    C: PixelColor,
{
    pub fn flush(&mut self) -> Result<(), TrySendError<DisplayAction<C>>> {
        self.send_pending()?;

        while self.channel.is_full() {}

        self.channel.try_send(DisplayAction::Flush)
    }

    /// Waits until the display task has drawn everything queued, so a full frame fits in the channel.
    pub async fn idle(&self) {
        while !self.channel.is_empty() {
            yield_now().await;
        }
    }

    fn send_pending(&mut self) -> Result<(), TrySendError<DisplayAction<C>>> {
        if self.pending.is_empty() {
            return Ok(());
        }

        self.channel
            .try_send(DisplayAction::Draw(core::mem::take(&mut self.pending)))
    }
}

impl<T, C, const CN: usize> Dimensions for DisplayProxy<'_, T, C, CN>
where
    C: PixelColor,
    T: RawMutex,
{
    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(Point::zero(), VISIBLE_AREA.size)
    }
}

impl<T, C, const CN: usize> DrawTarget for DisplayProxy<'_, T, C, CN>
where
    T: RawMutex,
    C: PixelColor,
{
    type Color = C;
    type Error = TrySendError<DisplayAction<C>>;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let offset = VISIBLE_AREA.top_left;

        for pixel in pixels
            .into_iter()
            .map(|Pixel(point, color)| Pixel(point + offset, color))
        {
            if !VISIBLE_AREA.contains(pixel.0) {
                continue;
            }

            if self.pending.is_full() {
                self.send_pending()?;
            }

            let _ = self.pending.push(pixel);
        }

        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let area = area.translate(VISIBLE_AREA.top_left).intersection(&VISIBLE_AREA);

        if area.is_zero_sized() {
            return Ok(());
        }

        self.send_pending()?;
        self.channel.try_send(DisplayAction::FillSolid(area, color))
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.pending.clear();
        self.channel.try_send(DisplayAction::Clear(color))
    }
}
