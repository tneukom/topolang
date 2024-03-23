use std::{collections::HashMap, ops::Index, path::Path};

use crate::{
    bitmap::Bitmap,
    field::{Field, FieldIndex},
    math::{
        pixel::Pixel,
        point::Point,
        rect::{Rect, RectBounds},
        rgba8::Rgba8,
    },
    topology::{Border, Interior},
    utils::IteratorPlus,
};

#[derive(Debug, Clone, Eq)]
pub struct Pixmap {
    field: Field<Option<Rgba8>>,
}

impl Pixmap {
    pub fn new(bounds: Rect<i64>) -> Self {
        Self {
            field: Field::filled(bounds, None),
        }
    }

    pub fn pixel_bounds<'a>(pixels: impl IntoIterator<Item = &'a Pixel>) -> Rect<i64> {
        RectBounds::iter_bounds(pixels.into_iter().map(|pixel| pixel.point())).inc_high()
    }

    pub fn from_hashmap(pixels: &HashMap<Pixel, Rgba8>) -> Self {
        let bounds = Self::pixel_bounds(pixels.keys());
        let mut field = Field::filled(bounds, None);
        for (&pixel, &color) in pixels {
            field.set(pixel, Some(color));
        }
        Self { field }
    }

    // TODO: Simplify
    pub fn from_bitmap(bitmap: &Bitmap) -> Self {
        let bounds = Rect::low_size([0, 0], [bitmap.width() as i64, bitmap.height() as i64]);
        let mut field = Field::filled(bounds, None);
        for index in bitmap.indices() {
            let color = bitmap[index];

            if color.a == 0 && color != Rgba8::TRANSPARENT {
                println!("Bitmap should not contain colors with alpha = 0 but rgb != 0")
            }

            field[index.as_i64()] = Some(color);
        }
        Self { field }
    }

    pub fn from_bitmap_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let bitmap = Bitmap::from_path(path)?;
        Ok(Self::from_bitmap(&bitmap))
    }

    /// Pixels outside the bitmap are ignored.
    /// TODO: Use Field<Rgba> instead of bitmap
    pub fn paint_to_bitmap(&self, bitmap: &mut Bitmap) {
        for (point, &color) in self.field.enumerate() {
            if let Some(color) = color {
                let index = point.as_usize();
                if bitmap.contains_index(index) {
                    bitmap[index] = color;
                }
            }
        }
    }

    // /// Default color is transparent, any pixels outside [0, size.x] x [0, size.y] are ignored.
    // pub fn to_bitmap_with_size(&self, size: Point<usize>) -> Bitmap {
    //     let mut bitmap = Bitmap::plain(size.x, size.y, Rgba8::BLACK);
    //     self.paint_to_bitmap(&mut bitmap);
    //     bitmap
    // }

    /// Translate pixmap to positive coordinates, top left pixel will be at (0, 0) and convert
    /// to bitmap.
    pub fn to_bitmap(&self) -> Bitmap {
        let rgba = self.field.map(|color| color.unwrap_or(Rgba8::BLACK));
        rgba.into_array2d()
    }

    pub fn without_color(mut self, removed: Rgba8) -> Self {
        for color in self.field.iter_mut() {
            if *color == Some(removed) {
                *color = None;
            }
        }
        self
    }

    pub fn without_void_color(self) -> Self {
        self.without_color(Rgba8::VOID)
    }

    pub fn get(&self, pixel: impl FieldIndex) -> Option<&Rgba8> {
        match self.field.get(pixel) {
            Some(color) => color.as_ref(),
            None => None,
        }
    }

    pub fn set(&mut self, pixel: Pixel, color: Rgba8) -> Option<Rgba8> {
        self.field.set(pixel, Some(color))
    }

    // pub fn remove(&mut self, pixel: &Pixel) -> Option<Rgba8> {
    //
    //     self.map.remove(pixel)
    // }

    pub fn keys<'a>(&'a self) -> impl IteratorPlus<Pixel> + 'a {
        self.field
            .indices()
            .filter_map(|index| match self.field.get(index) {
                Some(Some(_)) => Some(index.into()),
                _ => None,
            })
    }
    //
    // // TODO: Is this needed?
    // pub fn values(&self) -> impl IteratorPlus<&Rgba8> {
    //     self.map.values()
    // }

    // pub fn len(&self) -> usize {
    //     self.map.len()
    // }

    /// Entries right of boundary are removed
    /// TODO: Use border.bounds(), skip BTreeSet
    pub fn extract_right(&mut self, boundary: &Border) -> Pixmap {
        // Extract pixels left of inner_border
        let right_pixels = boundary.right_pixels();
        let bounds = Self::pixel_bounds(&right_pixels);
        let mut extracted = Field::filled(bounds, None);

        for pixel in right_pixels {
            extracted.set(pixel, self.field.set(pixel, None));
        }

        Self { field: extracted }
    }

    /// Upper bounds is exclusive, lower bound is inclusive
    pub fn bounds(&self) -> Rect<i64> {
        RectBounds::iter_bounds(self.field.indices()).inc_high()
    }

    pub fn translated(self, offset: Point<i64>) -> Self {
        Self {
            field: self.field.translated(offset),
        }
    }

    pub fn fill_interior(&mut self, interior: &Interior) {
        for &pixel in &interior.pixels {
            self.set(pixel, interior.color);
        }
    }

    pub fn blit(&mut self, other: &Pixmap) {
        self.field.blit(&other.field);
    }

    // pub fn retain(&mut self, f: impl FnMut(&Pixel, &mut Rgba8) -> bool) {
    //     self.map.retain(f)
    // }
}

impl PartialEq for Pixmap {
    fn eq(&self, other: &Self) -> bool {
        // TODO: Could this be done without iterating over lhs and rhs indices?
        self.field
            .indices()
            .chain(other.field.indices())
            .all(|index| self.get(index) == other.get(index))
    }
}

impl<Idx: FieldIndex> Index<Idx> for Pixmap {
    type Output = Rgba8;

    fn index(&self, pixel: Idx) -> &Self::Output {
        self.get(pixel).unwrap()
    }
}

// impl<'a> IntoIterator for &'a Pixmap {
//     type Item = (&'a Pixel, &'a Rgba8);
//     type IntoIter = hash_map::Iter<'a, Pixel, Rgba8>;
//
//     fn into_iter(self) -> Self::IntoIter {
//         self.map.iter()
//     }
// }
