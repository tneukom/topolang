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
    topology::Border,
    utils::IteratorPlus,
};

#[derive(Debug, Clone, Eq)]
pub struct Pixmap<T> {
    field: Field<Option<T>>,
}

pub type PixmapRgba = Pixmap<Rgba8>;

impl<T> Pixmap<T> {
    pub fn pixel_bounds<'a>(pixels: impl IntoIterator<Item = &'a Pixel>) -> Rect<i64> {
        RectBounds::iter_bounds(pixels.into_iter().map(|pixel| pixel.point())).inc_high()
    }

    pub fn get(&self, index: impl FieldIndex) -> Option<&T> {
        match self.field.get(index) {
            Some(value) => value.as_ref(),
            None => None,
        }
    }

    pub fn get_mut(&mut self, index: impl FieldIndex) -> Option<&mut T> {
        match self.field.get_mut(index) {
            Some(value) => value.as_mut(),
            None => None,
        }
    }

    pub fn set(&mut self, pixel: Pixel, value: T) -> Option<T> {
        self.field.set(pixel, Some(value))
    }

    pub fn map<S>(&self, mut f: impl FnMut(&T) -> S) -> Pixmap<S> {
        let field = self.field.map(|value| match value {
            Some(value) => Some(f(value)),
            None => None,
        });
        Pixmap { field }
    }

    pub fn filter_map<S>(&self, mut f: impl FnMut(&T) -> Option<S>) -> Pixmap<S> {
        let field = self.field.map(|value| match value {
            Some(value) => f(value),
            None => None,
        });
        Pixmap { field }
    }

    pub fn keys<'a>(&'a self) -> impl IteratorPlus<Pixel> + 'a {
        self.field
            .enumerate()
            .filter_map(|(index, value)| match value {
                Some(_) => Some(index.into()),
                None => None,
            })
    }

    pub fn iter<'a>(&'a self) -> impl IteratorPlus<(Pixel, &T)> + 'a {
        self.field
            .enumerate()
            .filter_map(|(index, value)| match value {
                Some(value) => Some((index.into(), value)),
                None => None,
            })
    }

    /// Upper bounds is exclusive, lower bound is inclusive
    /// Returns actual bounds
    pub fn actual_bounds(&self) -> Rect<i64> {
        RectBounds::iter_bounds(self.field.indices()).inc_high()
    }

    pub fn bounds(&self) -> Rect<i64> {
        self.field.bounds()
    }

    pub fn translated(self, offset: Point<i64>) -> Self {
        Self {
            field: self.field.translated(offset),
        }
    }

    pub fn field(&self) -> &Field<Option<T>> {
        &self.field
    }

    pub fn retain(&mut self, mut f: impl FnMut(Pixel, &mut T) -> bool) {
        for (index, opt_value) in self.field.enumerate_mut() {
            let keep = if let Some(value) = opt_value {
                f(index.into(), value)
            } else {
                false
            };

            if !keep {
                *opt_value = None;
            }
        }
    }
}

impl<T: Clone> Pixmap<T> {
    pub fn new(bounds: Rect<i64>) -> Self {
        Self {
            field: Field::filled(bounds, None),
        }
    }

    pub fn from_hashmap(pixels: &HashMap<Pixel, T>) -> Self {
        let bounds = Self::pixel_bounds(pixels.keys());
        let mut field = Field::filled(bounds, None);
        for (&pixel, value) in pixels {
            field.set(pixel, Some(value.clone()));
        }
        Self { field }
    }

    /// Entries right of boundary are removed
    /// TODO: Use border.bounds(), skip BTreeSet
    pub fn extract_right(&mut self, boundary: &Border) -> Pixmap<T> {
        // Extract pixels left of inner_border
        let right_pixels = boundary.right_pixels();
        let bounds = Self::pixel_bounds(&right_pixels);
        let mut extracted = Field::filled(bounds, None);

        for pixel in right_pixels {
            extracted.set(pixel, self.field.set(pixel, None));
        }

        Self { field: extracted }
    }

    pub fn blit(&mut self, other: &Pixmap<T>) {
        // Only overwrite when other[index] is Some
        for (index, value) in other.field.enumerate() {
            if value.is_some() {
                self.field.set(index, value.clone());
            }
        }
    }

    pub fn filter(&self, mut pred: impl FnMut(&T) -> bool) -> Self {
        self.filter_map(|value| {
            if pred(value) {
                Some(value.clone())
            } else {
                None
            }
        })
    }

    pub fn shrink(&self) -> Self {
        let bounds = self.actual_bounds();
        Pixmap {
            field: self.field.sub(bounds),
        }
    }

    pub fn clipped(&self, bounds: Rect<i64>) -> Self {
        Self {
            field: self.field.clipped(bounds),
        }
    }
}

impl Pixmap<Rgba8> {
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

    pub fn save(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let bitmap = self.to_bitmap();
        bitmap.save(path)
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
}

impl<T: Eq> Pixmap<T> {
    pub fn without_color(mut self, removed: &T) -> Self {
        for color in self.field.iter_mut() {
            if color.as_ref() == Some(removed) {
                *color = None;
            }
        }
        self
    }
}

impl<T: PartialEq> PartialEq for Pixmap<T> {
    fn eq(&self, other: &Self) -> bool {
        // TODO: Could this be done without iterating over lhs and rhs indices?
        self.field
            .indices()
            .chain(other.field.indices())
            .all(|index| self.get(index) == other.get(index))
    }
}

impl<Idx: FieldIndex, T> Index<Idx> for Pixmap<T> {
    type Output = T;

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
