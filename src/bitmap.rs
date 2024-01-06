use crate::{array_2d::Array2d, math::rgba8::Rgba8};
use arboard::ImageData;
use image::Rgba;
use std::{path::Path, usize};

pub type Bitmap = Array2d<Rgba8>;

impl Bitmap {
    pub fn plain(width: usize, height: usize, color: Rgba8) -> Self {
        Self::filled(width, height, &color)
    }

    pub fn transparent(width: usize, height: usize) -> Self {
        Self::plain(width, height, Rgba8::TRANSPARENT)
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, image::ImageError> {
        let imageio_bitmap = image::open(path)?.into_rgba8();
        let mut bitmap = Self::plain(
            imageio_bitmap.width() as usize,
            imageio_bitmap.height() as usize,
            Rgba8::ZERO,
        );
        for y in 0..bitmap.height() {
            for x in 0..bitmap.width() {
                let Rgba(rgba) = imageio_bitmap.get_pixel(x as u32, y as u32);
                bitmap[(x, y)] = Rgba8::from(*rgba);
            }
        }

        Ok(bitmap)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let mut imageio_bitmap = image::RgbaImage::new(self.width() as u32, self.height() as u32);

        for y in 0..self.height() {
            for x in 0..self.width() {
                let imageio_rgba = image::Rgba(self[[x, y]].to_array());
                imageio_bitmap.put_pixel(x as u32, y as u32, imageio_rgba);
            }
        }

        imageio_bitmap.save(path)?;
        Ok(())
    }

    pub fn is_plain(&self, color: Rgba8) -> bool {
        self.iter().all(|pixel| pixel == &color)
    }

    pub fn is_zero(&self) -> bool {
        self.is_plain(Rgba8::TRANSPARENT)
    }
}

impl<'a> From<&arboard::ImageData<'a>> for Bitmap {
    fn from(image: &ImageData) -> Self {
        let mut bitmap = Self::plain(image.width, image.height, Rgba8::ZERO);
        for y in 0..image.height {
            for x in 0..image.width {
                let offset = 4 * (y * image.width + x);
                let pixel_bytes: &[u8; 4] = image.bytes[offset..offset + 4].try_into().unwrap();
                bitmap[(x, y)] = Rgba8::from(*pixel_bytes);
            }
        }
        bitmap
    }
}
