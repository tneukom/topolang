use std::{
    io::Cursor,
    ops::{Index, IndexMut},
    path::Path,
};

use image::{Rgba, RgbaImage};

use crate::{
    material::Material,
    math::{point::Point, rect::Rect, rgba8::Rgba8},
    utils::IteratorPlus,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field<T> {
    bounds: Rect<i64>,
    elems: Vec<T>,
}

pub type RgbaField = Field<Rgba8>;
pub type MaterialField = Field<Material>;

impl<T> Field<T> {
    pub fn from_linear(bounds: Rect<i64>, elems: Vec<T>) -> Self {
        assert!(bounds.width() >= 0);
        assert!(bounds.height() >= 0);
        assert_eq!(bounds.width() * bounds.height(), elems.len() as i64);
        Self { bounds, elems }
    }

    pub fn from_map(bounds: Rect<i64>, f: impl FnMut(Point<i64>) -> T) -> Self {
        // Meaning bounds.width() > 0 and bounds.height() > 0
        // assert!(!bounds.is_empty());

        let mut elems = Vec::with_capacity(bounds.width() as usize * bounds.height() as usize);
        elems.extend(bounds.iter_half_open().map(f));
        Self::from_linear(bounds, elems)
    }

    pub fn linear_slice(&self) -> &[T] {
        &self.elems
    }

    pub fn bounds(&self) -> Rect<i64> {
        self.bounds
    }

    /// Always > 0
    pub fn width(&self) -> i64 {
        self.bounds.width()
    }

    /// Always > 0
    pub fn height(&self) -> i64 {
        self.bounds.height()
    }

    pub fn len(&self) -> usize {
        (self.bounds.width() * self.bounds().height()) as usize
    }

    pub fn indices(&self) -> impl IteratorPlus<Point<i64>> {
        self.bounds.iter_half_open()
    }

    pub fn enumerate(&self) -> impl IteratorPlus<(Point<i64>, &T)> {
        // FIXME: Should not require index check
        self.indices().map(|index| (index, &self[index]))
    }

    pub fn enumerate_mut(&mut self) -> impl Iterator<Item = (Point<i64>, &mut T)> {
        self.indices().zip(self.iter_mut())
    }

    pub fn contains_index(&self, index: impl FieldIndex) -> bool {
        self.bounds.contains(index.point())
    }

    pub fn linear_index(&self, index: impl FieldIndex) -> Option<usize> {
        if !self.bounds().half_open_contains(index.point()) {
            None
        } else {
            let i = (index.x() - self.bounds.x.low)
                + (index.y() - self.bounds.y.low) * self.bounds.width();
            Some(i as usize)
        }
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &T> {
        self.elems.iter()
    }

    pub fn iter_mut(&mut self) -> impl ExactSizeIterator<Item = &mut T> {
        self.elems.iter_mut()
    }

    pub fn as_slice(&self) -> &[T] {
        self.elems.as_slice()
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.elems.as_mut_slice()
    }

    pub fn get(&self, index: impl FieldIndex) -> Option<&T> {
        let i = self.linear_index(index)?;
        Some(&self.elems[i])
    }

    pub fn get_mut(&mut self, index: impl FieldIndex) -> Option<&mut T> {
        let i = self.linear_index(index)?;
        Some(&mut self.elems[i])
    }

    pub fn set(&mut self, index: impl FieldIndex, value: T) -> T {
        let i = self.linear_index(index).unwrap();
        std::mem::replace(&mut self.elems[i], value)
    }

    pub fn translated(self, offset: Point<i64>) -> Self {
        Self {
            bounds: self.bounds + offset,
            elems: self.elems,
        }
    }

    pub fn map<S>(&self, f: impl FnMut(&T) -> S) -> Field<S> {
        let elems = self.elems.iter().map(f).collect();
        Field {
            bounds: self.bounds,
            elems,
        }
    }

    pub fn into_map<S>(self, f: impl FnMut(T) -> S) -> Field<S> {
        // Will this reuse the Vec storage if sizeof(S) == sizeof(T)? See
        // https://stackoverflow.com/a/48309116
        // https://www.reddit.com/r/rust/comments/16hx79e/when_does_vecinto_itermapcollect_reallocate_and/
        // https://doc.rust-lang.org/nightly/src/alloc/vec/in_place_collect.rs.html
        let elems = self.elems.into_iter().map(f).collect();
        Field {
            bounds: self.bounds,
            elems,
        }
    }

    pub fn map_into<S: From<T>>(self) -> Field<S> {
        self.into_map(|value| value.into())
    }

    pub fn sub_map<S>(&self, bounds: Rect<i64>, mut f: impl FnMut(&T) -> S) -> Field<S> {
        assert!(self.bounds.contains_rect(bounds));
        Field::from_map(bounds, |index| f(&self[index]))
    }
}

impl<T: Clone> Field<T> {
    pub fn filled(bounds: Rect<i64>, value: T) -> Self {
        let size = bounds.width() as usize * bounds.height() as usize;
        Self {
            bounds,
            elems: vec![value; size],
        }
    }

    pub fn blit(&mut self, other: &Self) {
        for (index, value) in other.enumerate() {
            self.set(index, value.clone());
        }
    }

    pub fn sub(&self, bounds: Rect<i64>) -> Self {
        self.sub_map(bounds, |value| value.clone())
    }

    pub fn clipped(&self, bounds: Rect<i64>) -> Self {
        self.sub(bounds.intersect(self.bounds))
    }
}

impl RgbaField {
    fn from_imageio_bitmap(imageio_bitmap: &RgbaImage) -> Self {
        let mut bitmap = Self::filled(
            Rect::low_size(
                [0, 0],
                [
                    imageio_bitmap.width() as i64,
                    imageio_bitmap.height() as i64,
                ],
            ),
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

        for index in self.indices() {
            let imageio_rgba = image::Rgba(self[index].to_array());
            let offset = (index - self.bounds.low()).cwise_try_into::<u32>().unwrap();
            imageio_bitmap.put_pixel(offset.x, offset.y, imageio_rgba);
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

    pub fn into_material(self) -> MaterialField {
        self.into_map(|rgba| rgba.into())
    }
}

impl MaterialField {
    pub fn into_rgba(self) -> RgbaField {
        self.into_map(|material| material.into())
    }
}

pub trait FieldIndex: Copy {
    fn x(&self) -> i64;
    fn y(&self) -> i64;

    fn point(self) -> Point<i64> {
        Point::new(self.x(), self.y())
    }
}

impl<Idx: FieldIndex, T> Index<Idx> for Field<T> {
    type Output = T;

    fn index(&self, index: Idx) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<Idx: FieldIndex, T> IndexMut<Idx> for Field<T> {
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

impl FieldIndex for Point<i64> {
    fn x(&self) -> i64 {
        self.x
    }

    fn y(&self) -> i64 {
        self.y
    }
}

impl FieldIndex for [i64; 2] {
    fn x(&self) -> i64 {
        self[0]
    }

    fn y(&self) -> i64 {
        self[1]
    }
}

impl FieldIndex for (i64, i64) {
    fn x(&self) -> i64 {
        self.0
    }

    fn y(&self) -> i64 {
        self.1
    }
}
