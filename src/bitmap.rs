use crate::{array_2d::Array2d, math::rgba8::Rgba8};
// use arboard::ImageData;
use image::{Rgba, RgbaImage};
use std::{io::Cursor, path::Path, usize};

pub type Bitmap = Array2d<Rgba8>;

impl Bitmap {
    pub fn plain(width: usize, height: usize, color: Rgba8) -> Self {
        Self::filled(width, height, color)
    }

    pub fn transparent(width: usize, height: usize) -> Self {
        Self::plain(width, height, Rgba8::TRANSPARENT)
    }

    fn from_imageio_bitmap(imageio_bitmap: &RgbaImage) -> Self {
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

        bitmap
    }

    pub fn load_from_memory(memory: &[u8]) -> Result<Self, image::ImageError> {
        let imageio_bitmap = image::load_from_memory(memory)?.into_rgba8();
        Ok(Self::from_imageio_bitmap(&imageio_bitmap))
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, image::ImageError> {
        let imageio_bitmap = image::open(path)?.into_rgba8();
        Ok(Self::from_imageio_bitmap(&imageio_bitmap))
    }

    pub fn to_imageio(&self) -> image::RgbaImage {
        let mut imageio_bitmap = image::RgbaImage::new(self.width() as u32, self.height() as u32);

        for y in 0..self.height() {
            for x in 0..self.width() {
                let imageio_rgba = image::Rgba(self[[x, y]].to_array());
                imageio_bitmap.put_pixel(x as u32, y as u32, imageio_rgba);
            }
        }

        imageio_bitmap
    }

    pub fn save(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        self.to_imageio().save(path)?;
        Ok(())
    }

    pub fn to_png(&self) -> anyhow::Result<Vec<u8>> {
        let mut cursor = Cursor::new(Vec::new());
        self.to_imageio()
            .write_to(&mut cursor, image::ImageFormat::Png)?;
        Ok(cursor.into_inner())
    }

    pub fn is_plain(&self, color: Rgba8) -> bool {
        self.iter().all(|pixel| pixel == &color)
    }

    pub fn is_zero(&self) -> bool {
        self.is_plain(Rgba8::TRANSPARENT)
    }

    pub fn as_raw(&self) -> &[u8] {
        unsafe { self.as_slice().align_to::<u8>().1 }
    }
}

// TODO: Enable for non-wasm build
// impl<'a> From<&arboard::ImageData<'a>> for Bitmap {
//     fn from(image: &ImageData) -> Self {
//         let mut bitmap = Self::plain(image.width, image.height, Rgba8::ZERO);
//         for y in 0..image.height {
//             for x in 0..image.width {
//                 let offset = 4 * (y * image.width + x);
//                 let pixel_bytes: &[u8; 4] = image.bytes[offset..offset + 4].try_into().unwrap();
//                 bitmap[(x, y)] = Rgba8::from(*pixel_bytes);
//             }
//         }
//         bitmap
//     }
// }
