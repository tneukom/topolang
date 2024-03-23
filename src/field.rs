use std::ops::{Index, IndexMut};

use crate::{
    array_2d::Array2d,
    math::{pixel::Pixel, point::Point, rect::Rect},
    utils::IteratorPlus,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field<T> {
    bounds: Rect<i64>,
    elems: Vec<T>,
}

impl<T> Field<T> {
    pub fn from_linear(bounds: Rect<i64>, elems: Vec<T>) -> Self {
        // Meaning bounds.width() > 0 and bounds.height() > 0
        assert!(!bounds.is_empty());
        assert_eq!(bounds.width() * bounds.height(), elems.len() as i64);

        Self { bounds, elems }
    }

    pub fn from_map(bounds: Rect<i64>, f: impl Fn(Pixel) -> T) -> Self {
        // Meaning bounds.width() > 0 and bounds.height() > 0
        assert!(!bounds.is_empty());

        let mut elems = Vec::with_capacity(bounds.width() as usize * bounds.height() as usize);
        elems.extend(bounds.iter_half_open().map(|point| f(Pixel::from(point))));
        Self::from_linear(bounds, elems)
    }

    pub fn into_array2d(self) -> Array2d<T> {
        let size = self.bounds.size().as_usize();
        Array2d::from_linear(size.x, size.y, self.elems)
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

    pub fn indices(&self) -> impl IteratorPlus<Point<i64>> {
        self.bounds.iter_half_open()
    }

    pub fn enumerate(&self) -> impl IteratorPlus<(Point<i64>, &T)> {
        self.indices().map(|index| (index, &self[index]))
    }

    pub fn contains_index(&self, index: impl FieldIndex) -> bool {
        self.bounds.contains(index.into_point())
    }

    fn valid_linear_index_at(&self, index: impl FieldIndex) -> Option<usize> {
        if !self.bounds().half_open_contains(index.into_point()) {
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

    pub fn get(&self, index: impl FieldIndex) -> Option<&T> {
        let i = self.valid_linear_index_at(index)?;
        Some(&self.elems[i])
    }

    pub fn get_mut(&mut self, index: impl FieldIndex) -> Option<&mut T> {
        let i = self.valid_linear_index_at(index)?;
        Some(&mut self.elems[i])
    }

    pub fn set(&mut self, index: impl FieldIndex, value: T) -> T {
        let i = self.valid_linear_index_at(index).unwrap();
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
}

impl<T: Clone> Field<T> {
    pub fn filled(bounds: Rect<i64>, value: T) -> Self {
        assert!(!bounds.is_empty());
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
}

pub trait FieldIndex: Copy {
    fn x(&self) -> i64;
    fn y(&self) -> i64;

    fn into_point(self) -> Point<i64> {
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

impl FieldIndex for Pixel {
    fn x(&self) -> i64 {
        self.x
    }

    fn y(&self) -> i64 {
        self.y
    }
}
