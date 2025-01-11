use super::{
    point::Point,
    rect::{Rect, RectBounds},
};
use crate::math::generic::{Dot, FloatNum, Num, SignedNum};
use num_traits::{clamp, AsPrimitive};
use std::{clone::Clone, fmt::Debug};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Arrow<T> {
    pub a: Point<T>,
    pub b: Point<T>,
}

#[allow(non_snake_case)]
pub fn Arrow<T>(a: Point<T>, b: Point<T>) -> Arrow<T> {
    Arrow { a, b }
}

impl<T> Arrow<T> {
    pub fn corners(self) -> [Point<T>; 2] {
        [self.a, self.b]
    }

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

    pub fn swap_xy(self) -> Self {
        Self {
            a: self.a.swap_xy(),
            b: self.b.swap_xy(),
        }
    }
}

impl<T: Num> Arrow<T> {
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

    /// P - closest point on the line
    pub fn closest_point_offset(self, p: Point<T>) -> Point<T> {
        // Let A + tAB be the projection of p onto the line then we have
        // <P - (A + tAB), AB> = 0 and therefore
        // <P - A, AB> - t<AB, AB> = 0
        // t = <P - A, AB>/<AB, AB>
        let ap = p - self.a;
        let ab = self.dir();
        let t = ap.dot(ab) / ab.dot(ab);
        // If t is outside the range [0, 1] the closest point is the endpoint of the line segment.

        let t_clamped = clamp(t, T::ZERO, T::ONE);

        // A + t * AB is the closest point on the line to P
        // P - A + t * AB = AP - t * AB
        ap - ab * t_clamped
    }

    /// Returns the point on the line closest to p
    pub fn closest_point(self, p: Point<T>) -> Point<T> {
        self.closest_point_offset(p) + self.a
    }

    pub fn distance_squared(self, p: Point<T>) -> T {
        let offset = self.closest_point_offset(p);
        offset.norm_squared()
    }

    pub fn cwise_as<S>(self) -> Arrow<S>
    where
        T: AsPrimitive<S>,
        S: Copy + 'static,
    {
        Arrow {
            a: self.a.cwise_as(),
            b: self.b.cwise_as(),
        }
    }
}

impl<T: FloatNum> Arrow<T> {
    pub fn distance(self, p: Point<T>) -> T {
        self.distance_squared(p).sqrt()
    }
}

impl<T: SignedNum> Arrow<T> {
    pub fn mirror_x(self) -> Self {
        Self::new(self.a.mirror_x(), self.b.mirror_x())
    }

    pub fn mirror_y(self) -> Self {
        Self::new(self.a.mirror_y(), self.b.mirror_y())
    }
}
