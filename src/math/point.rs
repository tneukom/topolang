use crate::math::generic::{Dot, Num, SignedNum};
use num_traits::{real::Real, AsPrimitive};
use std::{
    cmp::Ordering,
    fmt::{Display, Formatter},
    hash::{Hash, Hasher},
    ops::{Add, Div, Mul, Neg, Rem, Sub},
};

#[derive(Copy, Clone, Debug)]
pub struct Point<T> {
    pub x: T,
    pub y: T,
}

#[allow(non_snake_case)]
pub const fn Point<T>(x: T, y: T) -> Point<T> {
    Point::new(x, y)
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

    pub fn cwise_as<S>(self) -> Point<S>
    where
        T: AsPrimitive<S>,
        S: Copy + 'static,
    {
        Point {
            x: self.x.as_(),
            y: self.y.as_(),
        }
    }

    pub fn swap_xy(self) -> Self {
        Self::new(self.y, self.x)
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
}

impl<T: AsPrimitive<f64>> Point<T> {
    pub fn as_f64(self) -> Point<f64> {
        self.cwise_as()
    }
}

impl<T: AsPrimitive<i64>> Point<T> {
    pub fn as_i64(self) -> Point<i64> {
        self.cwise_as()
    }
}

impl<T: SignedNum> Point<T> {
    // Return counterclockwise orthogonal vector (with x pointing right and y pointing up)
    // (1,0) -> (0,1)
    // (0,1) -> (-1,0)
    pub fn orthogonal_ccw(self) -> Self {
        Self::new(-self.y, self.x)
    }

    pub fn mirror_x(self) -> Self {
        Self::new(-self.x, self.y)
    }

    pub fn mirror_y(self) -> Self {
        Self::new(self.x, -self.y)
    }
}

impl<T: Real + Num> Point<T> {
    pub fn norm(self) -> T {
        self.norm_squared().sqrt()
    }

    pub fn distance(self, rhs: Self) -> T {
        self.distance_squared(rhs).sqrt()
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

impl<T: Display> Display for Point<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Point({}, {})", self.x, self.y)
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

impl<T> Add for Point<T>
where
    T: Add<Output = T>,
{
    type Output = Point<T>;

    fn add(self, rhs: Point<T>) -> Self::Output {
        Point::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<T> Add<T> for Point<T>
where
    T: Add<Output = T> + Copy,
{
    type Output = Point<T>;

    fn add(self, rhs: T) -> Self::Output {
        Point::new(self.x + rhs, self.y + rhs)
    }
}

impl<T> Sub for Point<T>
where
    T: Sub<Output = T>,
{
    type Output = Point<T>;

    fn sub(self, rhs: Point<T>) -> Self::Output {
        Point::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<T> Sub<T> for Point<T>
where
    T: Sub<Output = T> + Copy,
{
    type Output = Point<T>;

    fn sub(self, rhs: T) -> Self::Output {
        Point::new(self.x - rhs, self.y - rhs)
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
        Point::new(-self.x, -self.y)
    }
}

/// Right multiplication
impl<T> Mul<T> for Point<T>
where
    T: Mul<Output = T> + Copy,
{
    type Output = Point<T>;

    fn mul(self, rhs: T) -> Self::Output {
        Point::new(self.x * rhs, self.y * rhs)
    }
}

/// Left multiplication
macro_rules! impl_left_mul {
    ($t: ty) => {
        impl Mul<Point<$t>> for $t {
            type Output = Point<$t>;

            fn mul(self, rhs: Point<$t>) -> Self::Output {
                Point::new(self * rhs.x, self * rhs.y)
            }
        }
    };
}

impl_left_mul!(f32);
impl_left_mul!(f64);
impl_left_mul!(u8);
impl_left_mul!(u16);
impl_left_mul!(u32);
impl_left_mul!(u64);
impl_left_mul!(i8);
impl_left_mul!(i16);
impl_left_mul!(i32);
impl_left_mul!(i64);
impl_left_mul!(usize);
impl_left_mul!(isize);

impl<T> Div<T> for Point<T>
where
    T: Div<Output = T> + Copy,
{
    type Output = Point<T>;

    fn div(self, rhs: T) -> Self::Output {
        Point::new(self.x.div(rhs), self.y.div(rhs))
    }
}

impl<T> Rem<T> for Point<T>
where
    T: Rem<Output = T> + Copy,
{
    type Output = Point<T>;

    fn rem(self, rhs: T) -> Self::Output {
        Point::new(self.x.rem(rhs), self.y.rem(rhs))
    }
}

impl<T: PartialEq> PartialEq for Point<T> {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl<T: Eq> Eq for Point<T> {}

/// Top to bottom, left to right
impl<T: PartialOrd> PartialOrd for Point<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (&self.y, &self.x).partial_cmp(&(&other.y, &other.x))
    }
}

/// Top to bottom, left to right
impl<T: Ord> Ord for Point<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        (&self.y, &self.x).cmp(&(&other.y, &other.x))
    }
}

impl<T: Hash> Hash for Point<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.hash(state);
        self.y.hash(state);
    }
}

/// Warning: casts f64 to f32
impl From<Point<f64>> for egui::Pos2 {
    fn from(value: Point<f64>) -> Self {
        Self::new(value.x as f32, value.y as f32)
    }
}

/// Warning: casts f64 to f32
impl From<Point<f64>> for egui::Vec2 {
    fn from(value: Point<f64>) -> Self {
        Self::new(value.x as f32, value.y as f32)
    }
}

impl From<egui::Pos2> for Point<f64> {
    fn from(value: egui::Pos2) -> Self {
        Self::new(value.x as f64, value.y as f64)
    }
}

impl From<egui::Vec2> for Point<f64> {
    fn from(value: egui::Vec2) -> Self {
        Self::new(value.x as f64, value.y as f64)
    }
}
