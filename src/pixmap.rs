use crate::{
    bitmap::Bitmap,
    math::{
        pixel::Pixel,
        point::Point,
        rect::{Rect, RectBounds},
        rgba8::Rgba8,
    },
    topology::{Border, Region},
    utils::IteratorPlus,
};
use std::{
    collections::{btree_map, BTreeMap},
    ops::Index,
    path::Path,
};

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

    /// Pixels outside the bitmap are ignored.
    pub fn paint_to_bitmap(&self, bitmap: &mut Bitmap) {
        for (&pixel, &color) in &self.map {
            if let Ok(index) = pixel.point().cwise_try_into::<usize>() {
                if bitmap.contains_index(index) {
                    bitmap[index] = color;
                }
            }
        }
    }

    /// Default color is transparent, any pixels outside [0, size.x] x [0, size.y] are ignored.
    pub fn to_bitmap_with_size(&self, size: Point<usize>) -> Bitmap {
        let mut bitmap = Bitmap::plain(size.x, size.y, Rgba8::BLACK);
        self.paint_to_bitmap(&mut bitmap);
        bitmap
    }

    /// Translate pixmap to positive coordinates, top left pixel will be at (0, 0) and convert
    /// to bitmap.
    pub fn to_bitmap(&self) -> Bitmap {
        let translated = self.translated(-self.bounds().low());
        let size = translated
            .bounds()
            .size()
            .cwise_try_into::<usize>()
            .unwrap();
        translated.to_bitmap_with_size(size)
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

    pub fn keys(&self) -> impl IteratorPlus<&Pixel> {
        self.map.keys()
    }

    pub fn values(&self) -> impl IteratorPlus<&Rgba8> {
        self.map.values()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// If fill is None the entries right of boundary are removed otherwise they are set to the fill color.
    pub fn extract_right(&mut self, boundary: &Border, fill: Option<Rgba8>) -> Pixmap {
        // Extract pixels left of inner_border
        let mut right = BTreeMap::new();
        for pixel in boundary.right_pixels() {
            let color = if let Some(fill) = fill {
                self.map.insert(pixel, fill).unwrap()
            } else {
                self.map.remove(&pixel).unwrap()
            };
            right.insert(pixel, color);
        }

        Self { map: right }
    }

    /// Upper bounds is exclusive, lower bound is inclusive
    pub fn bounds(&self) -> Rect<i64> {
        RectBounds::iter_bounds(self.keys().map(|pixel| pixel.point())).inc_high()
    }

    pub fn translated(&self, offset: Point<i64>) -> Self {
        let map = self
            .map
            .iter()
            .map(|(&pixel, &color)| (pixel + offset, color))
            .collect();
        Self { map }
    }

    pub fn fill_region(&mut self, region: &Region) {
        for &pixel in &region.interior {
            self.set(pixel, region.color);
        }
    }

    pub fn blit(&mut self, other: &Pixmap) {
        for (&pixel, &color) in other {
            self.set(pixel, color)
        }
    }

    pub fn retain(&mut self, f: impl FnMut(&Pixel, &mut Rgba8) -> bool) {
        self.map.retain(f)
    }
}

impl Index<&Pixel> for Pixmap {
    type Output = Rgba8;

    fn index(&self, pixel: &Pixel) -> &Self::Output {
        &self.map[pixel]
    }
}

impl<'a> IntoIterator for &'a Pixmap {
    type Item = (&'a Pixel, &'a Rgba8);
    type IntoIter = btree_map::Iter<'a, Pixel, Rgba8>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}
