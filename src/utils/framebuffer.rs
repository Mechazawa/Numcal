use core::convert::Infallible;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

const WIDTH: u32 = 128;
const HEIGHT: u32 = 32;
const ROW_BYTES: usize = WIDTH.div_ceil(8) as usize;

/// A 1-bit frame the size of the visible display, drawn in memory and sent as a whole.
pub struct Framebuffer([[u8; ROW_BYTES]; HEIGHT as usize]);

impl Framebuffer {
    pub const fn new() -> Self {
        Self([[0; ROW_BYTES]; HEIGHT as usize])
    }

    fn index(point: Point) -> Option<(usize, usize, u8)> {
        let x = u32::try_from(point.x).ok().filter(|&x| x < WIDTH)? as usize;
        let y = u32::try_from(point.y).ok().filter(|&y| y < HEIGHT)? as usize;

        Some((y, x / 8, 0x80 >> (x % 8)))
    }

    pub fn invert(&mut self, area: &Rectangle) {
        area.points()
            .filter_map(Self::index)
            .for_each(|(row, byte, mask)| self.0[row][byte] ^= mask);
    }

    pub fn lit_pixels(&self) -> impl Iterator<Item = Pixel<BinaryColor>> + '_ {
        self.bounding_box()
            .points()
            .filter(|&point| Self::index(point).is_some_and(|(row, byte, mask)| self.0[row][byte] & mask != 0))
            .map(|point| Pixel(point, BinaryColor::On))
    }
}

impl OriginDimensions for Framebuffer {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for Framebuffer {
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
}
