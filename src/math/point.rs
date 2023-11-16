use crate::math::generic::{
    CwiseAdd, CwiseDiv, CwiseEuclidDivRem, CwiseInv, CwiseMul, CwiseSub, Dot, EuclidDivRem,
    IntoLossy, Num, SignedNum,
};
use num_traits::{real::Real, Inv};
use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    ops::{Add, Div, Mul, Neg, Sub},
};

#[derive(Copy, Clone, Debug)]
pub struct Point<T> {
    pub x: T,
    pub y: T,
}

pub fn pt<T>(x: T, y: T) -> Point<T> {
    Point::new(x, y)
}

pub fn pt_from<T>(x: impl Into<T>, y: impl Into<T>) -> Point<T> {
    Point::new(x.into(), y.into())
}

impl<T> Point<T> {
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    pub fn new_from(x: impl Into<T>, y: impl Into<T>) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
        }
    }

    pub fn to_array(self) -> [T; 2] {
        [self.x, self.y]
    }

    pub fn cwise_into<S>(self) -> Point<S>
    where
        T: Into<S>,
    {
        Point {
            x: self.x.into(),
            y: self.y.into(),
        }
    }

    pub fn cwise_into_lossy<S>(self) -> Point<S>
    where
        T: IntoLossy<S>,
    {
        Point {
            x: self.x.into_lossy(),
            y: self.y.into_lossy(),
        }
    }

    pub fn cwise_try_into<S>(self) -> Result<Point<S>, <T as TryInto<S>>::Error>
    where
        T: TryInto<S>,
    {
        Ok(Point {
            x: self.x.try_into()?,
            y: self.y.try_into()?,
        })
    }

    pub fn swap_xy(self) -> Self {
        Self {
            x: self.y,
            y: self.x,
        }
    }
}

impl<T: Num> Point<T> {
    pub fn norm_squared(self) -> T {
        self.dot(self)
    }

    pub fn distance_squared(self, rhs: Self) -> T {
        (rhs - self).norm_squared()
    }

    pub const E_X: Self = Point::new(T::ONE, T::ZERO);
    pub const E_Y: Self = Point::new(T::ZERO, T::ONE);
    pub const ZERO: Self = Point::new(T::ZERO, T::ZERO);
    pub const ONE: Self = Point::new(T::ONE, T::ONE);

    pub fn inf_norm(self) -> T {
        self.x.abs().max(self.y.abs())
    }

    pub fn cwise_min(self, rhs: Self) -> Self {
        Self {
            x: self.x.min(rhs.x),
            y: self.y.min(rhs.y),
        }
    }

    pub fn cwise_max(self, rhs: Self) -> Self {
        Self {
            x: self.x.max(rhs.x),
            y: self.y.max(rhs.y),
        }
    }
}

impl<T: SignedNum> Point<T> {
    // Return counter clockwise orthogonal vector (with x pointing right and y pointing up)
    // (1,0) -> (0,1)
    // (0,1) -> (-1,0)
    pub fn orthogonal_ccw(self) -> Self {
        Self::new(-self.y, self.x)
    }
}

impl<T: Ord> Point<T> {
    pub fn cmp_lexical(&self, rhs: &Self) -> Ordering {
        match self.x.cmp(&rhs.x) {
            Ordering::Equal => self.y.cmp(&rhs.y),
            other => other,
        }
    }

    // Is this < q in lexicographical order, meaning first compare real than imag.
    pub fn less_lexical(&self, rhs: &Self) -> bool {
        self.cmp_lexical(rhs).is_lt()
    }
}

// impl<T: RoundToInt> Point<T> {
//     pub fn floor_i64(self) -> Point<i64> {
//         Point::new(self.x.floor_i64(), self.y.floor_i64())
//     }
//
//     pub fn round_i64(self) -> Point<i64> {
//         Point::new(self.x.round_i64(), self.y.round_i64())
//     }
//
//     pub fn ceil_i64(self) -> Point<i64> {
//         Point::new(self.x.ceil_i64(), self.y.ceil_i64())
//     }
// }

impl Point<usize> {
    pub fn to_f64(self) -> Point<f64> {
        Point::new(self.x as f64, self.y as f64)
    }
}

impl<T: Real + Num> Point<T> {
    pub fn norm(self) -> T {
        self.norm_squared().sqrt()
    }

    pub fn normalized(self) -> Self {
        self / self.norm()
    }

    pub fn round(self) -> Self {
        Self::new(self.x.round(), self.y.round())
    }

    pub fn ceil(self) -> Self {
        Self::new(self.x.ceil(), self.y.ceil())
    }

    pub fn floor(self) -> Self {
        Self::new(self.x.floor(), self.y.floor())
    }
}

impl<T> From<(T, T)> for Point<T> {
    fn from(value: (T, T)) -> Self {
        Self::new(value.0, value.1)
    }
}

/// The more general version From<[S; 2]> where T: From<S> make type inference work less good.
impl<T> From<[T; 2]> for Point<T> {
    fn from(value: [T; 2]) -> Self {
        let [x, y] = value;
        Self::new(x, y)
    }
}

// impl<T, S> From<[S; 2]> for Point<T>
// where
//     T: From<S>,
// {
//     fn from(value: [S; 2]) -> Self {
//         let [x, y] = value;
//         Self::new(x.into(), y.into())
//     }
// }

impl<Lhs, Rhs> Add<Point<Rhs>> for Point<Lhs>
where
    Lhs: Add<Rhs>,
{
    type Output = Point<<Lhs as Add<Rhs>>::Output>;

    fn add(self, rhs: Point<Rhs>) -> Self::Output {
        Self::Output::new(self.x + rhs.x, self.y + rhs.y)
    }
}

// impl<Lhs, Rhs> AddAssign<Point<Rhs>> for Point<Lhs>
// where
//     Lhs: Add<Rhs, Output = Lhs> + Copy,
// {
//     fn add_assign(&mut self, rhs: Point<Rhs>) {
//         *self = *self + rhs;
//     }
// }

impl<Lhs, Rhs> Sub<Point<Rhs>> for Point<Lhs>
where
    Lhs: Sub<Rhs>,
{
    type Output = Point<<Lhs as Sub<Rhs>>::Output>;

    fn sub(self, rhs: Point<Rhs>) -> Self::Output {
        Self::Output::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<T> Dot<Point<T>> for Point<T>
where
    T: Add<Output = T> + Mul<Output = T>,
{
    type Output = T;

    fn dot(self, rhs: Self) -> Self::Output {
        self.x * rhs.x + self.y * rhs.y
    }
}

impl<T> Neg for Point<T>
where
    T: Neg<Output = T>,
{
    type Output = Point<T>;

    fn neg(self) -> Self::Output {
        Self::Output::new(-self.x, -self.y)
    }
}

impl<T> CwiseInv for Point<T>
where
    T: Inv<Output = T>,
{
    type Output = Point<T>;

    fn cwise_inv(self) -> Self::Output {
        Self::Output::new(self.x.inv(), self.y.inv())
    }
}

impl<Lhs, Rhs> CwiseAdd<Rhs> for Point<Lhs>
where
    Lhs: Add<Rhs>,
    Rhs: Copy,
{
    type Output = Point<<Lhs as Add<Rhs>>::Output>;

    fn cwise_add(self, rhs: Rhs) -> Self::Output {
        Self::Output::new(self.x + rhs, self.y + rhs)
    }
}

impl<Lhs, Rhs> CwiseSub<Rhs> for Point<Lhs>
where
    Lhs: Sub<Rhs>,
    Rhs: Copy,
{
    type Output = Point<<Lhs as Sub<Rhs>>::Output>;

    fn cwise_sub(self, rhs: Rhs) -> Self::Output {
        Self::Output::new(self.x - rhs, self.y - rhs)
    }
}

impl<Lhs, Rhs> Mul<Rhs> for Point<Lhs>
where
    Lhs: Mul<Rhs>,
    Rhs: Copy,
{
    type Output = Point<<Lhs as Mul<Rhs>>::Output>;

    fn mul(self, rhs: Rhs) -> Self::Output {
        Self::Output::new(self.x * rhs, self.y * rhs)
    }
}

impl<Lhs, Rhs> Div<Rhs> for Point<Lhs>
where
    Lhs: Div<Rhs>,
    Rhs: Copy,
{
    type Output = Point<<Lhs as Div<Rhs>>::Output>;

    fn div(self, rhs: Rhs) -> Self::Output {
        Self::Output::new(self.x / rhs, self.y / rhs)
    }
}

impl<Lhs, Rhs> CwiseMul<Point<Rhs>> for Point<Lhs>
where
    Lhs: Mul<Rhs>,
{
    type Output = Point<<Lhs as Mul<Rhs>>::Output>;

    fn cwise_mul(self, rhs: Point<Rhs>) -> Self::Output {
        Self::Output::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl<Lhs, Rhs> CwiseDiv<Point<Rhs>> for Point<Lhs>
where
    Lhs: Div<Rhs>,
{
    type Output = Point<<Lhs as Div<Rhs>>::Output>;

    fn cwise_div(self, rhs: Point<Rhs>) -> Self::Output {
        Self::Output::new(self.x / rhs.x, self.y / rhs.y)
    }
}

impl<Lhs, Rhs> CwiseEuclidDivRem<Point<Rhs>> for Point<Lhs>
where
    Lhs: EuclidDivRem<Rhs>,
{
    type Output = Point<<Lhs as EuclidDivRem<Rhs>>::Output>;

    fn cwise_euclid_div(self, rhs: Point<Rhs>) -> Self::Output {
        Self::Output::new(self.x.euclid_div(rhs.x), self.y.euclid_div(rhs.y))
    }

    fn cwise_euclid_rem(self, rhs: Point<Rhs>) -> Self::Output {
        Self::Output::new(self.x.euclid_rem(rhs.x), self.y.euclid_rem(rhs.y))
    }
}

impl<T: PartialEq> PartialEq for Point<T> {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl<T: Eq> Eq for Point<T> {}

impl<T: PartialOrd> PartialOrd for Point<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (&self.x, &self.y).partial_cmp(&(&other.x, &other.y))
    }
}

impl<T: Ord> Ord for Point<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        (&self.x, &self.y).cmp(&(&other.x, &other.y))
    }
}

impl<T: Hash> Hash for Point<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.hash(state);
        self.y.hash(state);
    }
}

/// Convert from Point<f32> to egui
impl From<Point<f32>> for egui::Pos2 {
    fn from(value: Point<f32>) -> Self {
        Self::new(value.x, value.y)
    }
}

/// Convert from egui to Point<f32>
impl From<egui::Pos2> for Point<f32> {
    fn from(value: egui::Pos2) -> Self {
        Self::new(value.x, value.y)
    }
}
