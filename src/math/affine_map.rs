use std::{
    fmt::Debug,
    ops::{Add, Mul},
};

use crate::math::{
    generic::{FloatNum, Num},
    matrix2::Matrix2,
    point::Point,
    rect::{Rect, RectBounds},
};

#[derive(Debug, Clone, Copy)]
pub struct AffineMap<T> {
    pub linear: Matrix2<T>,
    pub constant: Point<T>,
}

impl<T> AffineMap<T> {
    pub const fn new(linear: Matrix2<T>, constant: Point<T>) -> Self {
        Self { linear, constant }
    }
}

impl<T: Num> AffineMap<T> {
    pub const fn linear(linear: Matrix2<T>) -> Self {
        Self::new(linear, Point::ZERO)
    }

    pub const fn translation(translation: Point<T>) -> Self {
        Self::new(Matrix2::ID, translation)
    }

    pub const fn similarity(scale: T, constant: Point<T>) -> Self {
        let linear = Matrix2::diagonal(scale, scale);
        Self { linear, constant }
    }

    pub const ID: Self = Self::new(Matrix2::ID, Point::ZERO);
}

impl<T: FloatNum> AffineMap<T> {
    // lInv = l^-1, tInv = -lInv * t
    // (AInv * A) * p = lInv * l * p + lInv * t + (-lInv * t) = p
    pub fn inv(self) -> Self {
        let linear_inv = self.linear.inv();
        let constant_inv = -(linear_inv * self.constant);
        Self {
            linear: linear_inv,
            constant: constant_inv,
        }
    }

    pub fn map_dirs(
        a: Point<T>,
        ta: Point<T>,
        u: Point<T>,
        tu: Point<T>,
        v: Point<T>,
        tv: Point<T>,
    ) -> Self {
        let linear = Matrix2::map_basis(u, tu, v, tv);
        //Tv = Lv + v0 => v0 = Tv - Lv
        let constant = ta - (linear * a);
        Self { linear, constant }
    }

    pub fn map_points(
        a: Point<T>,
        ta: Point<T>,
        b: Point<T>,
        tb: Point<T>,
        c: Point<T>,
        tc: Point<T>,
    ) -> Self {
        let u = b - a;
        let tu = tb - ta; // = Lv1 + v0 - Lv - v0 = L(v1 - v) = Ldir1
        let v = c - a;
        let tv = tc - ta; // = Lv2 + v0 - Lv - v0 = L(v2 - v) = Ldir2

        Self::map_dirs(a, ta, u, tu, v, tv)
    }

    pub fn map_rect(rect: Rect<T>, phi_rect: Rect<T>) -> Self {
        Self::map_points(
            rect.top_left(),
            phi_rect.top_left(),
            rect.bottom_left(),
            phi_rect.bottom_left(),
            rect.top_right(),
            phi_rect.top_right(),
        )
    }

    // pub fn is_orthogonal(&self) -> bool {
    //     self.linear.is_orthogonal()
    // }
    //
    // /// Maps axis aligned rectangles to axis aligned rectangles.
    // pub fn is_axis_aligned_rect_map(&self) -> bool {
    //     self.is_orthogonal() && (self.linear.col1().is_perp())
    // }
}

impl<T> Mul<Point<T>> for AffineMap<T>
where
    T: Copy + Mul<Output = T> + Add<Output = T>,
{
    type Output = Point<T>;

    fn mul(self, v: Point<T>) -> Point<T> {
        self.linear * v + self.constant
    }
}

/// The result is the axis aligned bounding rectangle of the true (self * rhs)
impl<T> Mul<Rect<T>> for AffineMap<T>
where
    T: Num,
{
    type Output = Rect<T>;

    fn mul(self, rhs: Rect<T>) -> Self::Output {
        let phi_top_left = self * rhs.top_left();
        let phi_bottom_left = self * rhs.bottom_left();
        let phi_top_right = self * rhs.top_right();
        [phi_top_left, phi_bottom_left, phi_top_right].bounds()
    }
}

impl<T> Mul for AffineMap<T>
where
    T: Copy + Mul<Output = T> + Add<Output = T>,
{
    type Output = Self;

    //(AL * AR) * p = lL * (lR * p + tR) + tL = lL * lR * p + lL * tR + tL
    fn mul(self, rhs: Self) -> Self::Output {
        Self::Output {
            linear: self.linear * rhs.linear,
            constant: self.linear * rhs.constant + self.constant,
        }
    }
}
