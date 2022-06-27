use crate::math::{
    axis_line::AxisArrow,
    generic::{EuclidDivRem, Num, SignedNum},
    point::Point,
    rect::{Rect, RectBounds},
};
use num_traits::Inv;
use std::{hash::Hash, ops::Mul};

/// Rotations around multiples of 90°
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum Turn {
    Ccw0 = 0,
    Ccw90 = 1,
    Ccw180 = 2,
    Ccw270 = 3,
}

impl Turn {
    pub const ID: Self = Self::Ccw0;

    pub const CW_90: Self = Self::Ccw270;
    pub const CW_180: Self = Self::Ccw180;
    pub const CW_270: Self = Self::Ccw90;

    pub const ALL: [Self; 4] = [Self::Ccw0, Self::Ccw90, Self::Ccw180, Self::Ccw270];

    pub fn is_even(self) -> bool {
        self == Self::Ccw0 || self == Self::Ccw180
    }

    pub fn is_odd(self) -> bool {
        !self.is_even()
    }

    /// Rotates indices in an array of the given size.
    /// 0,0
    ///  ┌────┐
    ///  │    │
    ///  └────┘
    ///      w,h
    pub fn rotate_array_index<T: Num>(self, size: Point<T>, index: Point<T>) -> Point<T> {
        // flip_x(x, y) = (size.x - 1 - x, y)
        // flip_y(x, y) = (x, size.y - 1 - y)
        // transpose(x, y) = (y, x)
        match self {
            Self::Ccw0 => index,
            // Ccw 90 = transpose * flip_x
            Self::Ccw90 => Point::new(index.y, size.x - T::ONE - index.x),
            // Ccw 180 = flip_y * flip_x
            Self::Ccw180 => Point::new(size.x - T::ONE - index.x, size.y - T::ONE - index.y),
            // Ccw 270 = transpose * flip_y
            Self::Ccw270 => Point::new(size.y - T::ONE - index.y, index.x),
        }
    }

    /// Rotate a half open rectangle of array indices
    /// The set of indices in the resulting rectangle (half open) is the same as
    /// {rotate_array_index(size, index) for index in rect}
    pub fn rotate_array_index_rect<T: Num>(self, size: Point<T>, rect: Rect<T>) -> Rect<T> {
        let min = rect.low();
        let max = rect.high() - Point::ONE;
        let turned_min = self.rotate_array_index(size, min);
        let turned_max = self.rotate_array_index(size, max);
        [turned_min, turned_max].bounds().inc_high()
    }

    /// Returns scaling' such that
    /// Diag(scaling') = Rot(turn)^-1 * Diag(scaling) * Rot(turn)
    pub fn conjugate_scaling<T: Num>(self, scaling: Point<T>) -> Point<T> {
        // Let R = ccw(90°), a short calculation shows
        // R^-1 * Diag(scale) * R * p = (scale_y * x, scale_x * y)
        // Inductively R^-n * Diag(scale_x, scale_y) * R^n = Diag(scale_x, scale_y) if n even and
        // Diag(scale_y, scale_x) if n odd
        if self.is_even() {
            scaling
        } else {
            scaling.swap_xy()
        }
    }
}

impl From<i64> for Turn {
    fn from(value: i64) -> Self {
        match value.euclid_rem(4) {
            0 => Self::Ccw0,
            1 => Self::Ccw90,
            2 => Self::Ccw180,
            3 => Self::Ccw270,
            _ => unreachable!(),
        }
    }
}

impl Inv for Turn {
    type Output = Self;

    fn inv(self) -> Self::Output {
        Self::from(-(self as i64))
    }
}

impl Mul for Turn {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::from(self as i64 + rhs as i64)
    }
}

/// Rotate the point around the origin. Keep in mind that the y axis points downwards and the x axis
/// to the right.
impl<Rhs: SignedNum> Mul<Point<Rhs>> for Turn {
    type Output = Point<Rhs>;

    fn mul(self, rhs: Point<Rhs>) -> Self::Output {
        match self {
            Self::Ccw0 => rhs,
            Self::Ccw90 => Point::new(rhs.y, -rhs.x),
            Self::Ccw180 => Point::new(-rhs.x, -rhs.y),
            Self::Ccw270 => Point::new(-rhs.y, rhs.x),
        }
    }
}

impl<Rhs: SignedNum> Mul<Rect<Rhs>> for Turn {
    type Output = Rect<Rhs>;

    fn mul(self, rhs: Rect<Rhs>) -> Self::Output {
        // TODO: For performance implement without bounds, match self, we know what axis to flip
        if rhs.is_empty() {
            rhs
        } else {
            [self * rhs.low(), self * rhs.high()].bounds()
        }
    }
}

/// Same as AxisArrow::from_points(self * rhs.start_point(), self * rhs.stop_point())
impl<Rhs: SignedNum> Mul<AxisArrow<Rhs>> for Turn {
    type Output = AxisArrow<Rhs>;

    fn mul(self, rhs: AxisArrow<Rhs>) -> Self::Output {
        // TODO: For performance implement without from_points
        AxisArrow::from_points(self * rhs.start_point(), self * rhs.stop_point()).unwrap()
    }
}

#[cfg(test)]
mod test {
    use crate::math::{rect::Rect, turn::Turn};

    #[test]
    fn transform_rect() {
        let rect = Rect::low_high([0, 0], [3, 3]);
        let expected = Rect::low_high([-3, -3], [0, 0]);
        assert_eq!(Turn::Ccw180 * rect, expected);
    }
}
