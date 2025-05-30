use embedded_graphics_core::{
    Pixel,
    draw_target::DrawTarget,
    pixelcolor::Rgb888,
    prelude::{OriginDimensions, RgbColor, Size},
};
use limine::framebuffer::Framebuffer as LimineFramebuffer;

pub enum Error {
    OutOfBounds,
}

pub struct Framebuffer<'a> {
    inner: LimineFramebuffer<'a>,
}

impl<'a> Framebuffer<'a> {
    pub fn new(inner: LimineFramebuffer<'a>) -> Self {
        Self { inner }
    }
}

impl<'a> DrawTarget for Framebuffer<'a> {
    type Color = Rgb888;
    type Error = Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = Pixel<Rgb888>>,
    {
        let x_scale = self.inner.bpp() as usize / 8;
        let y_scale = self.inner.pitch() as usize;
        for Pixel(point, color) in pixels {
            if point.x < 0
                || point.y < 0
                || point.x as u64 >= self.inner.width()
                || point.y as u64 >= self.inner.height()
            {
                Err(Error::OutOfBounds)?;
            }
            let color_u32 = ((color.r() as u32) << self.inner.red_mask_shift())
                | ((color.g() as u32) << self.inner.green_mask_shift())
                | ((color.b() as u32) << self.inner.blue_mask_shift());
            unsafe {
                self.inner
                    .addr()
                    .add(point.x as usize * x_scale as usize + point.y as usize * y_scale as usize)
                    .cast::<u32>()
                    .write(color_u32);
            }
        }
        Ok(())
    }
}

impl<'a> OriginDimensions for Framebuffer<'a> {
    fn size(&self) -> Size {
        Size {
            width: self.inner.width() as u32,
            height: self.inner.height() as u32,
        }
    }
}

impl<'a> From<LimineFramebuffer<'a>> for Framebuffer<'a> {
    fn from(inner: LimineFramebuffer<'a>) -> Self {
        Self::new(inner)
    }
}
