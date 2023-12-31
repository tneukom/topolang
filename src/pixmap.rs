use crate::{
    bitmap::Bitmap,
    math::{pixel::Pixel, point::Point, rgba8::Rgba8},
    topology::Border,
};
use std::{collections::BTreeMap, ops::Index, path::Path};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pixmap {
    pub map: BTreeMap<Pixel, Rgba8>,
}

impl Pixmap {
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    pub fn from_bitmap(bitmap: &Bitmap) -> Self {
        let mut map: BTreeMap<Pixel, Rgba8> = BTreeMap::new();
        for idx in bitmap.indices() {
            let color = bitmap[idx];

            if color.a == 0 && color != Rgba8::TRANSPARENT {
                println!("Bitmap should not contain colors with alpha = 0 but rgb != 0")
            }

            let pixel: Pixel = idx.cwise_try_into::<i64>().unwrap().into();
            map.insert(pixel, color);
        }
        Self { map }
    }

    pub fn from_bitmap_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let bitmap = Bitmap::from_path(path)?;
        Ok(Self::from_bitmap(&bitmap))
    }

    /// Pixels outside of the bitmap are ignored.
    pub fn paint_to_bitmap(&self, bitmap: &mut Bitmap) {
        for (&pixel, &color) in &self.map {
            if let Ok(index) = pixel.point().cwise_try_into::<usize>() {
                if bitmap.contains_index(index) {
                    bitmap[index] = color;
                }
            }
        }
    }

    /// Default color is transparent.
    pub fn to_bitmap_with_size(&self, bounds: Point<usize>) -> Bitmap {
        let mut bitmap = Bitmap::plain(bounds.x, bounds.y, Rgba8::TRANSPARENT);
        self.paint_to_bitmap(&mut bitmap);
        bitmap
    }

    pub fn without_color(mut self, color: Rgba8) -> Self {
        self.map.retain(|_, &mut pixel_color| pixel_color != color);
        self
    }

    pub fn without_void_color(self) -> Self {
        self.without_color(Rgba8::VOID)
    }

    pub fn get(&self, pixel: &Pixel) -> Option<&Rgba8> {
        self.map.get(pixel)
    }

    pub fn set(&mut self, pixel: Pixel, color: Rgba8) {
        self.map.insert(pixel, color);
    }

    pub fn remove(&mut self, pixel: &Pixel) -> Option<Rgba8> {
        self.map.remove(pixel)
    }

    pub fn keys(&self) -> impl Iterator<Item = &Pixel> + Clone {
        self.map.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &Rgba8> + Clone {
        self.map.values()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn extract_right(&mut self, boundary: &Border) -> Pixmap {
        // Extract pixels left of inner_border
        let mut right = BTreeMap::new();
        for pixel in boundary.right_pixels() {
            let color = self.remove(&pixel).unwrap();
            right.insert(pixel, color);
        }

        Self { map: right }
    }
}

impl Index<&Pixel> for Pixmap {
    type Output = Rgba8;

    fn index(&self, pixel: &Pixel) -> &Self::Output {
        &self.map[pixel]
    }
}
