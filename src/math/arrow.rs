use super::{
    point::Point,
    rect::{Rect, RectBounds},
};
use crate::math::generic::Num;
use std::{clone::Clone, fmt::Debug};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Arrow<T> {
    pub a: Point<T>,
    pub b: Point<T>,
}

impl<T> Arrow<T> {
    pub fn corners(self) -> [Point<T>; 2] {
        [self.a, self.b]
    }
}

impl<T: Num> Arrow<T> {
    pub const fn new(a: Point<T>, b: Point<T>) -> Self {
        Self { a, b }
    }

    pub fn new_from(a: impl Into<Point<T>>, b: impl Into<Point<T>>) -> Self {
        Self::new(a.into(), b.into())
    }

    pub fn reversed(self) -> Self {
        Self {
            a: self.b,
            b: self.a,
        }
    }

    /// Returns an endpoint q with p != q,
    pub fn other_endpoint(self, p: Point<T>) -> Point<T> {
        if self.a == p {
            self.b
        } else {
            self.a
        }
    }

    pub fn is_endpoint(&self, p: Point<T>) -> bool {
        self.a == p || self.b == p
    }

    pub fn dir(self) -> Point<T> {
        self.b - self.a
    }

    pub fn at(self, t: T) -> Point<T> {
        self.a + self.dir() * t
    }

    pub fn length_squared(self) -> T {
        self.dir().norm_squared()
    }

    pub fn bounds(self) -> Rect<T> {
        self.corners().bounds()
    }
}
