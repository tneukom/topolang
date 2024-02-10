use super::{
    point::Point,
    rect::{Rect, RectBounds},
};
use crate::math::generic::{Dot, Num};
use num_traits::clamp;
use std::{clone::Clone, fmt::Debug};
use crate::math::generic::MinimumMaximum;

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

    pub const fn endpoints(self) -> [Point<T>; 2] {
        [self.a, self.b]
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

    pub fn distance_squared_to_rect_boundary(self, rect: Rect<T>) -> T {
        // Let P be on the rect boundary and Q on the line such that |P - Q| minimal.
        // A) P is a rect corner: Minimum of distances of rect corners to side
        // B) Q is a line endpoint: Minimum of endpoint to rect distances
        // C) Otherwise: Line must be parallel to one of the sides of the rectangle. We move PQ
        //    to and endpoint/corner while keeping it parallel, reducing this case to case A or B

        let endpoint_dist_sq = self
            .endpoints()
            .map(|endpoint| rect.distance_squared(endpoint))
            .minimum()
            .unwrap();

        let corner_dist_sq = rect
            .corners()
            .map(|corner| self.distance_squared(corner))
            .minimum()
            .unwrap();

        [endpoint_dist_sq, corner_dist_sq].minimum().unwrap()
    }
}
