use std::{
    fmt::Debug,
    ops::{Add, Mul, Sub},
};

use crate::math::{
    arrow::Arrow,
    generic::{Dot, FloatNum, Num, SignedNum},
    point::Point,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Matrix2<T> {
    pub a11: T,
    pub a12: T,
    pub a21: T,
    pub a22: T,
}

impl<T> Matrix2<T> {
    pub const fn new(a11: T, a12: T, a21: T, a22: T) -> Self {
        Self { a11, a12, a21, a22 }
    }
}

impl<T> Matrix2<T>
where
    T: Copy,
{
    pub const fn from_cols(col1: Point<T>, col2: Point<T>) -> Self {
        Self::new(col1.x, col2.x, col1.y, col2.y)
    }

    pub const fn from_rows(row1: Point<T>, row2: Point<T>) -> Self {
        Self::new(row1.x, row1.y, row2.x, row2.y)
    }

    pub const fn row1(self) -> Point<T> {
        Point::new(self.a11, self.a12)
    }

    pub const fn row2(self) -> Point<T> {
        Point::new(self.a21, self.a22)
    }

    pub const fn col1(self) -> Point<T> {
        Point::new(self.a11, self.a21)
    }

    pub const fn col2(self) -> Point<T> {
        Point::new(self.a12, self.a22)
    }

    pub const fn transpose(self) -> Self {
        Self::new(self.a11, self.a21, self.a12, self.a22)
    }

    // to array
    pub const fn to_array(self) -> [T; 4] {
        [self.a11, self.a12, self.a21, self.a22]
    }
}

impl<T: Num> Matrix2<T> {
    pub const ID: Self = Self::new(T::ONE, T::ZERO, T::ZERO, T::ONE);
    pub const ZERO: Self = Self::new(T::ZERO, T::ZERO, T::ZERO, T::ZERO);
    pub const SWAP_XY: Self = Self::new(T::ZERO, T::ONE, T::ONE, T::ZERO);

    pub const fn diagonal(a11: T, a22: T) -> Self {
        Self::new(a11, T::ZERO, T::ZERO, a22)
    }

    pub const fn diagonal_vec2(u: Point<T>) -> Self {
        Self::new(u.x, T::ZERO, T::ZERO, u.y)
    }

    pub fn det(self) -> T {
        self.a11 * self.a22 - self.a12 * self.a21
    }
}

impl<T: FloatNum> Matrix2<T> {
    pub fn norm_l1(self) -> T {
        self.a11.abs() + self.a12.abs() + self.a21.abs() + self.a22.abs()
    }
}

impl<T: SignedNum> Matrix2<T> {
    /// CCW with window coordinate system (x right, y down).
    /// Maps (1, 0) -> (0, -1) and (0, 1) -> (1, 0)
    pub fn ccw_90() -> Self {
        Self::new(T::ZERO, T::ONE, -T::ONE, T::ZERO)
    }

    /// Maps (1, 0) -> (-1, 0) and (0, 1) -> (0, 1)
    pub fn mirror_x() -> Self {
        Self::diagonal(-T::ONE, T::ONE)
    }

    pub fn mirror_y() -> Self {
        Self::diagonal(T::ONE, -T::ONE)
    }
}

impl<T: FloatNum> Matrix2<T> {
    pub fn inv(self) -> Self {
        let det = self.det();
        assert_ne!(det, T::ZERO);
        let inv_det = T::ONE / det;
        Self::new(
            self.a22 * inv_det,
            -self.a12 * inv_det,
            -self.a21 * inv_det,
            self.a11 * inv_det,
        )
    }

    //Returns a Matrix2 T s.t T * b1 = Tb1 && T * b2 = Tb2
    pub fn map_basis(b1: Point<T>, tb1: Point<T>, b2: Point<T>, tb2: Point<T>) -> Self {
        //from_columns(b1, b2).Inverse() maps b1 to (1,0) and b2 to (0,1)
        Self::from_cols(tb1, tb2) * Self::from_cols(b1, b2).inv()
    }
}

impl Matrix2<f64> {
    // to f32 array
    pub const fn to_f32_array(self) -> [f32; 4] {
        [
            self.a11 as f32,
            self.a12 as f32,
            self.a21 as f32,
            self.a22 as f32,
        ]
    }

    pub fn norm_l2(self) -> f64 {
        f64::sqrt(
            self.a11 * self.a11 + self.a12 * self.a12 + self.a21 * self.a21 + self.a22 * self.a22,
        )
    }
}

impl<T> Mul<T> for Matrix2<T>
where
    T: Copy + Mul<Output = T>,
{
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Matrix2::new(
            self.a11 * rhs,
            self.a12 * rhs,
            self.a21 * rhs,
            self.a22 * rhs,
        )
    }
}

impl<T> Mul<Point<T>> for Matrix2<T>
where
    T: Copy + Mul<Output = T> + Add<Output = T>,
{
    type Output = Point<T>;

    fn mul(self, rhs: Point<T>) -> Point<T> {
        Point::new(
            self.a11 * rhs.x + self.a12 * rhs.y,
            self.a21 * rhs.x + self.a22 * rhs.y,
        )
    }
}

impl<T> Mul<Arrow<T>> for Matrix2<T>
where
    T: Copy + Mul<Output = T> + Add<Output = T>,
{
    type Output = Arrow<T>;

    fn mul(self, rhs: Arrow<T>) -> Arrow<T> {
        Arrow::new(self * rhs.a, self * rhs.b)
    }
}

impl<T> Mul for Matrix2<T>
where
    T: Copy + Mul<Output = T> + Add<Output = T>,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let a11 = self.row1().dot(rhs.col1());
        let a12 = self.row1().dot(rhs.col2());
        let a21 = self.row2().dot(rhs.col1());
        let a22 = self.row2().dot(rhs.col2());
        Matrix2::new(a11, a12, a21, a22)
    }
}

impl<T> Add for Matrix2<T>
where
    T: Copy + Add<Output = T>,
{
    type Output = Matrix2<T>;

    fn add(self, rhs: Self) -> Self::Output {
        Matrix2::new(
            self.a11 + rhs.a11,
            self.a12 + rhs.a12,
            self.a21 + rhs.a21,
            self.a22 + rhs.a22,
        )
    }
}

impl<T> Sub for Matrix2<T>
where
    T: Copy + Sub<Output = T>,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Matrix2::new(
            self.a11 - rhs.a11,
            self.a12 - rhs.a12,
            self.a21 - rhs.a21,
            self.a22 - rhs.a22,
        )
    }
}
