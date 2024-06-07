use super::{
    point::Point,
    rect::{Rect, RectBounds},
};
use crate::math::generic::{Cast, Dot, Num, SignedNum};
use num_traits::clamp;
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

    pub fn swap_xy(self) -> Self {
        Self {
            a: self.a.swap_xy(),
            b: self.b.swap_xy(),
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

    pub fn cwise_into<S>(self) -> Arrow<S>
    where
        T: Into<S>,
    {
        Arrow {
            a: self.a.cwise_into(),
            b: self.b.cwise_into(),
        }
    }

    pub fn cwise_cast<S>(self) -> Arrow<S>
    where
        T: Cast<S>,
    {
        Arrow {
            a: self.a.cwise_cast(),
            b: self.b.cwise_cast(),
        }
    }

    pub fn cwise_try_into<S>(self) -> Result<Arrow<S>, <T as TryInto<S>>::Error>
    where
        T: TryInto<S>,
    {
        Ok(Arrow {
            a: self.a.cwise_try_into()?,
            b: self.b.cwise_try_into()?,
        })
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

impl Arrow<i64> {
    /// Only works if 0 < dir.x and 0 <= dir.y <= dir.x
    fn draw_impl0(self) -> Vec<Point<i64>> {
        let dir = self.dir();
        assert!(0 <= dir.y);
        assert!(0 < dir.x);
        assert!(dir.y <= dir.x);

        let slope = dir.y as f64 / dir.x as f64;

        let mut points: Vec<Point<i64>> = Vec::new();
        for x_offset in 0..=dir.x {
            let y_offset = (slope * (x_offset as f64)).round() as i64;
            let point = self.a + Point(x_offset, y_offset);
            // Make sure line is contiguous
            if let Some(previous) = points.last().copied() {
                if previous.y < point.y {
                    points.push(Point(previous.x, previous.y + 1))
                }
            }
            points.push(point);
        }

        points
    }

    /// Only works if 0 < dir.x and |dir.y| <= dir.x
    fn draw_impl1(self) -> Vec<Point<i64>> {
        assert!(0 < self.dir().x);
        assert!(self.dir().y.abs() <= self.dir().x);

        if self.dir().y < 0 {
            let mut points = self.mirror_y().draw_impl0();
            for point in &mut points {
                *point = point.mirror_y();
            }
            points
        } else {
            self.draw_impl0()
        }
    }

    /// Only works if dir.x != 0 and |dir.y| <= |dir.x|
    fn draw_impl2(self) -> Vec<Point<i64>> {
        assert_ne!(self.dir().x, 0);
        assert!(self.dir().y.abs() <= self.dir().x.abs());

        if self.dir().x > 0 {
            self.draw_impl1()
        } else {
            self.reversed().draw_impl1()
        }
    }

    /// Returns pixels on the line. Pixels are contiguous, in other words the following situation
    /// should not happen:
    /// ┌─┬─┐
    /// └─┴─┼─┬─┐
    ///     └─┴─┘
    /// Instead it should look like this
    /// ┌─┬─┐
    /// └─┼─┼─┬─┐
    ///   └─┴─┴─┘
    /// https://en.wikipedia.org/wiki/Line_drawing_algorithm
    pub fn draw(self) -> Vec<Point<i64>> {
        let dir = self.dir();
        if dir.x == 0 && dir.y == 0 {
            vec![self.a]
        } else if dir.y.abs() <= dir.x.abs() {
            self.draw_impl2()
        } else {
            let mut points = self.swap_xy().draw_impl2();
            for point in &mut points {
                *point = point.swap_xy();
            }
            points
        }
    }
}
