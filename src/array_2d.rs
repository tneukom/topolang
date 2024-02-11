use crate::{
    math::{point::Point, rect::Rect, turn::Turn},
    utils::IteratorPlus,
};
use num_traits::Inv;
use std::ops::{Index, IndexMut};

/// Derived Ord is lexicographical on fields from top to bottom, i.e. (width, height, elems)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Array2d<T> {
    width: usize,
    height: usize,

    elems: Vec<T>,
}

impl<T> Array2d<T> {
    pub const fn empty() -> Self {
        Self {
            width: 0,
            height: 0,
            elems: Vec::new(),
        }
    }

    pub fn from_linear(width: usize, height: usize, elems: Vec<T>) -> Self {
        Self {
            width,
            height,
            elems,
        }
    }

    pub fn from_map(size: impl Index2d, f: impl Fn(Point<usize>) -> T) -> Self {
        let mut elems = Vec::with_capacity(size.x() * size.y());
        elems.extend(Self::indices_for_size(size).map(f));
        Self::from_linear(size.x(), size.y(), elems)
    }

    pub fn linear_slice(&self) -> &[T] {
        &self.elems
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn columns(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn rows(&self) -> usize {
        self.height
    }

    pub fn is_empty(&self) -> bool {
        self.elems.is_empty()
    }

    pub fn size(&self) -> Point<usize> {
        Point::new(self.width, self.height)
    }

    pub fn indices_for_size(size: impl Index2d) -> impl IteratorPlus<Point<usize>> {
        Rect::low_size([0, 0], [size.x(), size.y()]).iter_half_open()
    }

    /// Iterator of indices (0, 0), (1, 0), (2, 0), ..., (0, 1), ...
    pub fn indices(&self) -> impl IteratorPlus<Point<usize>> {
        Self::indices_for_size(self.size())
    }

    pub fn enumerate(&self) -> impl IteratorPlus<(Point<usize>, &T)> {
        self.indices().map(|index| (index, &self[index]))
    }

    pub fn contains_index(&self, index: impl Index2d) -> bool {
        index.x() < self.width && index.y() < self.height
    }

    fn linear_index_at(&self, index: impl Index2d) -> usize {
        index.y() * self.width + index.x()
    }

    /// Iterate linearly over the given rectangle (half open indices)
    pub fn iter_sub_rect(&self, rect: impl Into<Rect<usize>>) -> impl IteratorPlus<&T> {
        let rect = rect.into();
        rect.iter_half_open().map(|idx| &self[idx])
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
}

impl<T: Clone> Array2d<T> {
    pub fn filled(width: usize, height: usize, item: &T) -> Self {
        let mut elems = Vec::with_capacity(width * height);
        for _ in 0..(width * height) {
            elems.push(item.clone())
        }
        Self {
            width,
            height,
            elems,
        }
    }

    /// Blit to `src[src_x..src_x+width, src_y..src_y+height]` self at position (x, y)
    pub fn blit_sub(
        &mut self,
        src: &Self,
        dst_point: impl Into<Point<usize>>,
        src_point: impl Into<Point<usize>>,
        size: impl Into<Point<usize>>,
    ) {
        let (src_point, dst_point, size) = (src_point.into(), dst_point.into(), size.into());

        assert!(src_point.x + size.x <= src.width());
        assert!(src_point.y + size.y <= src.height());
        assert!(dst_point.x + size.x <= self.width());
        assert!(dst_point.y + size.y <= self.height());

        for delta_y in 0..size.y {
            for delta_x in 0..size.x {
                let delta = Point::new(delta_x, delta_y);
                self[dst_point + delta] = src[src_point + delta].clone();
            }
        }
    }

    /// Blit other to self at position (dst_x, dst_y)
    pub fn blit(&mut self, src: &Self, dst_point: impl Into<Point<usize>>) {
        self.blit_sub(src, dst_point, [0usize, 0usize], src.size());
    }

    pub fn sub_rect(&self, rect: impl Into<Rect<usize>>) -> Self {
        let rect = rect.into();
        let mut elems = Vec::with_capacity(rect.width() * rect.height());
        elems.extend(self.iter_sub_rect(rect).cloned());
        Self::from_linear(rect.width(), rect.height(), elems)
    }

    pub fn transpose(&self) -> Self {
        Self::from_map(self.size().swap_xy(), |index| self[index.swap_xy()].clone())
    }

    pub fn flip_x(&self) -> Self {
        Self::from_map(self.size(), |index| {
            self[[self.width() - 1 - index.x, index.y]].clone()
        })
    }

    /// Turn the array, meaning `self[(x, y)]` is rotated to
    /// `result[turn.rotate_array_index((x, y))]`
    pub fn turn_by(&self, turn: Turn) -> Self {
        let size = if turn == Turn::Ccw0 || turn == Turn::Ccw180 {
            self.size()
        } else {
            self.size().swap_xy()
        };

        let inv_turn = turn.inv();
        Self::from_map(size, |index| {
            self[inv_turn.rotate_array_index(size, index)].clone()
        })
    }
}

impl<T, const WIDTH: usize, const HEIGHT: usize> From<[[T; WIDTH]; HEIGHT]> for Array2d<T> {
    fn from(value: [[T; WIDTH]; HEIGHT]) -> Self {
        let elems: Vec<_> = value.into_iter().flatten().collect();
        Self::from_linear(WIDTH, HEIGHT, elems)
    }
}

impl<Idx: Index2d, T> Index<Idx> for Array2d<T> {
    type Output = T;

    fn index(&self, index: Idx) -> &Self::Output {
        let index = self.linear_index_at(index);
        &self.elems[index]
    }
}

impl<Idx: Index2d, T> IndexMut<Idx> for Array2d<T> {
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output {
        let index = self.linear_index_at(index);
        &mut self.elems[index]
    }
}

pub trait Index2d: Sized + Copy + Eq {
    fn x(self) -> usize;
    fn y(self) -> usize;

    fn column(self) -> usize {
        self.x()
    }

    fn row(self) -> usize {
        self.y()
    }

    fn into_point(self) -> Point<usize> {
        Point::new(self.x(), self.y())
    }
}

impl Index2d for (usize, usize) {
    fn x(self) -> usize {
        self.0
    }

    fn y(self) -> usize {
        self.1
    }
}

impl Index2d for [usize; 2] {
    fn x(self) -> usize {
        self[0]
    }

    fn y(self) -> usize {
        self[1]
    }
}

impl Index2d for Point<usize> {
    fn x(self) -> usize {
        self.x
    }

    fn y(self) -> usize {
        self.y
    }
}

#[cfg(test)]
mod test {
    use crate::{
        array_2d::Array2d,
        math::{point::Point, turn::Turn},
    };
    use std::collections::HashMap;

    #[test]
    pub fn indices() {
        let indices: Vec<_> = Array2d::<()>::indices_for_size([3, 2]).collect();
        let expected: Vec<Point<usize>> = vec![
            Point(0, 0),
            Point(1, 0),
            Point(2, 0),
            Point(0, 1),
            Point(1, 1),
            Point(2, 1),
        ];
        assert_eq!(indices, expected);
    }

    #[test]
    pub fn turn() {
        // rot 0°
        // 1 1
        // 0 1
        let rot_0 = Array2d::from([[1, 1], [0, 1]]);

        // rot 90°
        // 1 1
        // 1 0
        let rot_90 = Array2d::from([[1, 1], [1, 0]]);

        // rot 180°
        // 1 0
        // 1 1
        let rot_180 = Array2d::from([[1, 0], [1, 1]]);

        // rot 270°
        // 0 1
        // 1 1
        let rot_270 = Array2d::from([[0, 1], [1, 1]]);

        assert_eq!(rot_0.turn_by(Turn::Ccw90), rot_90);
        assert_eq!(rot_0.turn_by(Turn::Ccw180), rot_180);
        assert_eq!(rot_0.turn_by(Turn::Ccw270), rot_270);
        assert_eq!(rot_0.turn_by(Turn::Ccw0), rot_0);

        let expected: HashMap<_, _> = [
            (Turn::Ccw0, rot_0),
            (Turn::Ccw90, rot_90),
            (Turn::Ccw180, rot_180),
            (Turn::Ccw270, rot_270),
        ]
        .into();

        for (&turn, array) in expected.iter() {
            for rhs in Turn::ALL {
                let turned_array = array.turn_by(rhs);
                let expected = &expected[&(rhs * turn)];
                assert_eq!(&turned_array, expected)
            }
        }
    }

    #[test]
    pub fn transpose_and_flip() {
        // Base array
        // 1 0 1
        // 1 1 1
        // 0 0 1
        let base = Array2d::from([[1, 0, 1], [1, 1, 1], [0, 0, 1]]);

        // Transposed array
        // 1 1 0
        // 0 1 0
        // 1 1 1
        let transpose = Array2d::from([[1, 1, 0], [0, 1, 0], [1, 1, 1]]);

        // Flip x array
        // 1 0 1
        // 1 1 1
        // 1 0 0
        let flip_x = Array2d::from([[1, 0, 1], [1, 1, 1], [1, 0, 0]]);

        assert_eq!(base.transpose(), transpose);
        assert_eq!(base.flip_x(), flip_x);
    }
}
