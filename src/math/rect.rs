use crate::math::{arrow::Arrow, generic::Num, interval::Interval, point::Point};
use itertools::Itertools;
use num_traits::{AsPrimitive, ConstZero};
use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    ops::{Add, Mul, Range, RangeInclusive, Sub},
};

///  low
///   ┌─────┐
///   │     │
///   └─────┘
///        high
///
///  top left       top right
///      ┌──────────────┐
///      │              │
///      │              │
///      └──────────────┘
/// bottom left   bottom right
#[derive(Clone, Copy, Debug)]
pub struct Rect<T> {
    pub x: Interval<T>,
    pub y: Interval<T>,
}

impl<T: Copy> Rect<T> {
    pub const fn new(x: Interval<T>, y: Interval<T>) -> Self {
        Self { x, y }
    }

    pub const fn intervals(x: Interval<T>, y: Interval<T>) -> Self {
        Self { x, y }
    }

    pub const fn low_high(low: Point<T>, high: Point<T>) -> Self {
        Self {
            x: Interval::new(low.x, high.x),
            y: Interval::new(low.y, high.y),
        }
    }

    pub fn cwise_as<S>(self) -> Rect<S>
    where
        T: AsPrimitive<S>,
        S: Copy + 'static,
    {
        Rect {
            x: self.x.cwise_as(),
            y: self.y.cwise_as(),
        }
    }

    pub fn low(&self) -> Point<T> {
        Point::new(self.x.low, self.y.low)
    }

    pub fn high(&self) -> Point<T> {
        Point::new(self.x.high, self.y.high)
    }

    pub fn top_right(&self) -> Point<T> {
        Point::new(self.x.high, self.y.low)
    }

    pub fn top_left(&self) -> Point<T> {
        Point::new(self.x.low, self.y.low)
    }

    pub fn bottom_left(&self) -> Point<T> {
        Point::new(self.x.low, self.y.high)
    }

    pub fn bottom_right(&self) -> Point<T> {
        Point::new(self.x.high, self.y.high)
    }

    /// Counter-clockwise
    pub fn corners(&self) -> [Point<T>; 4] {
        [
            self.top_left(),
            self.bottom_left(),
            self.bottom_right(),
            self.top_right(),
        ]
    }

    pub fn top(&self) -> T {
        self.y.low
    }

    pub fn bottom(&self) -> T {
        self.y.high
    }

    pub fn left(&self) -> T {
        self.x.low
    }

    pub fn right(&self) -> T {
        self.x.high
    }

    pub fn point(p: Point<T>) -> Self {
        Self::new(Interval::point(p.x), Interval::point(p.y))
    }

    pub const TRIANGLE_INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];
}

impl<T: Num> Rect<T> {
    pub const EMPTY: Self = Self::new(Interval::<T>::EMPTY, Interval::<T>::EMPTY);
    pub const UNIT: Self = Self::new(Interval::<T>::UNIT, Interval::<T>::UNIT);

    pub fn cell_rect(index: Point<T>) -> Rect<T> {
        Rect::low_size(index, Point::ONE)
    }

    pub fn is_empty(self) -> bool {
        self.x.is_empty() || self.y.is_empty()
    }

    pub fn is_point(self) -> bool {
        self.x.is_point() && self.y.is_point()
    }

    pub fn has_zero_area(self) -> bool {
        self.x.has_zero_length() || self.y.has_zero_length()
    }

    pub fn has_positive_area(self) -> bool {
        !self.has_zero_area()
    }

    /// p in [x.low, x.high] x [y.low, y.high]
    pub fn contains(self, p: Point<T>) -> bool {
        self.x.contains(p.x) && self.y.contains(p.y)
    }

    /// p in [x.low, x.high) x [y.low, y.high)
    pub fn half_open_contains(self, p: Point<T>) -> bool {
        self.x.half_open_contains(p.x) && self.y.half_open_contains(p.y)
    }

    pub fn clamp(self, p: Point<T>) -> Point<T> {
        Point::new(self.x.clamp(p.x), self.y.clamp(p.y))
    }

    pub fn bounds_with_rect(self, other: Self) -> Self {
        Self::new(
            self.x.bounds_with_interval(other.x),
            self.y.bounds_with_interval(other.y),
        )
    }

    pub fn bounds_with(self, other: Point<T>) -> Self {
        Self::new(
            self.x.bounds_with_value(other.x),
            self.y.bounds_with_value(other.y),
        )
    }

    /// Returns the smallest rectangle `rect` such that `rect.half_open_contains(index)` for all
    /// indices.
    pub fn index_bounds(indices: impl IntoIterator<Item = Point<T>>) -> Self {
        RectBounds::iter_bounds(indices.into_iter()).inc_high()
    }

    pub fn intersect(self, rhs: Self) -> Self {
        Self::new(self.x.intersect(rhs.x), self.y.intersect(rhs.y))
    }

    pub fn intersects(self, rhs: Self) -> bool {
        self.x.intersects(rhs.x) && self.y.intersects(rhs.y)
    }

    pub fn ccw_left_arrow(self) -> Arrow<T> {
        Arrow::new(self.top_left(), self.bottom_left())
    }

    pub fn ccw_bottom_arrow(self) -> Arrow<T> {
        Arrow::new(self.bottom_left(), self.bottom_right())
    }

    pub fn ccw_right_arrow(self) -> Arrow<T> {
        Arrow::new(self.bottom_right(), self.top_right())
    }

    pub fn ccw_top_arrow(self) -> Arrow<T> {
        Arrow::new(self.top_right(), self.top_left())
    }

    pub fn ccw_side_arrows(self) -> [Arrow<T>; 4] {
        [
            self.ccw_left_arrow(),
            self.ccw_bottom_arrow(),
            self.ccw_right_arrow(),
            self.ccw_top_arrow(),
        ]
    }

    //size has to be positive (size.X >= 0, size.Y >= 0)
    pub fn low_size(low: Point<T>, size: Point<T>) -> Self {
        Self::low_high(low, low + size)
    }

    pub fn size(self) -> Point<T> {
        Point::new(self.x.length(), self.y.length())
    }

    pub fn width(self) -> T {
        self.x.length()
    }

    pub fn height(self) -> T {
        self.y.length()
    }

    pub fn area(self) -> T {
        self.x.length() * self.y.length()
    }

    pub fn padded(self, padding: T) -> Self {
        Self::new(self.x.padded(padding), self.y.padded(padding))
    }

    /// Add one cwise to high, useful for indexing half open / closed rectangles
    pub fn inc_high(self) -> Self {
        Self::new(self.x.inc_high(), self.y.inc_high())
    }

    pub fn contains_point(self, p: Point<T>) -> bool {
        self.x.contains(p.x) && self.y.contains(p.y)
    }

    pub fn contains_rect(self, rect: Self) -> bool {
        if rect.is_empty() {
            return true;
        }
        self.x.contains_interval(rect.x) && self.y.contains_interval(rect.y)
    }

    pub fn distance_squared(self, p: Point<T>) -> T {
        self.clamp(p).distance_squared(p)
    }

    pub fn center(self) -> Point<T> {
        Point::new(self.x.center(), self.y.center())
    }
}

/// Right hand Point add
impl<T> Add<Point<T>> for Rect<T>
where
    T: Add<Output = T> + Copy,
{
    type Output = Rect<T>;

    fn add(self, rhs: Point<T>) -> Self::Output {
        Self::Output::new(self.x + rhs.x, self.y + rhs.y)
    }
}

/// Right hand Point sub
impl<T> Sub<Point<T>> for Rect<T>
where
    T: Sub<Output = T> + Copy,
{
    type Output = Rect<T>;

    fn sub(self, rhs: Point<T>) -> Self::Output {
        Self::Output::new(self.x - rhs.x, self.y - rhs.y)
    }
}

/// Right hand scalar mul, panics if rhs <= 0
impl<T> Mul<T> for Rect<T>
where
    T: Mul<Output = T> + Copy + PartialOrd + ConstZero,
{
    type Output = Rect<T>;

    fn mul(self, rhs: T) -> Self::Output {
        assert!(rhs > T::ZERO);
        Self::Output::new(self.x * rhs, self.y * rhs)
    }
}

impl<T: Num> PartialEq for Rect<T> {
    fn eq(&self, other: &Self) -> bool {
        (self.is_empty() && other.is_empty()) || (self.x == other.x && self.y == other.y)
    }
}

impl<T: Num> Eq for Rect<T> {}

impl<T: Num + Hash> Hash for Rect<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Don't forget the case where one interval is empty but the other isn't!
        if self.is_empty() {
            // hash some random number
            943923891.hash(state)
        } else {
            // Both x and y interval are not empty
            (self.x, self.y).hash(state)
        }
    }
}

pub trait RectBounds<T: Num>: Sized {
    fn bounds(self) -> Rect<T>;

    /// Return an empty Rect if the iter is empty
    fn iter_bounds(iter: impl Iterator<Item = Self>) -> Rect<T> {
        iter.map(Self::bounds)
            .fold(Rect::EMPTY, Rect::bounds_with_rect)
    }
}

impl<T: Num> RectBounds<T> for Point<T> {
    fn bounds(self) -> Rect<T> {
        Rect::point(self)
    }
}

impl<T: Num> RectBounds<T> for Rect<T> {
    fn bounds(self) -> Rect<T> {
        self
    }
}

impl<T: Num, E, const N: usize> RectBounds<T> for [E; N]
where
    E: RectBounds<T>,
{
    fn bounds(self) -> Rect<T> {
        self.into_iter()
            .map(E::bounds)
            .reduce(Rect::bounds_with_rect)
            .unwrap()
    }
}

/// Warning: casts f64 to f32
impl From<Rect<f64>> for egui::Rect {
    fn from(value: Rect<f64>) -> Self {
        Self::from_min_max(value.low().into(), value.high().into())
    }
}

impl From<egui::Rect> for Rect<f64> {
    fn from(value: egui::Rect) -> Self {
        let low: Point<f64> = value.min.into();
        let high: Point<f64> = value.max.into();
        Self::low_high(low, high)
    }
}

impl<T> Rect<T>
where
    T: Clone,
    Range<T>: Clone + Iterator<Item = T>,
    RangeInclusive<T>: Clone + Iterator<Item = T>,
{
    /// All whole number points in [x.low, x.high) x [y.low, y.high)
    pub fn iter_half_open(self) -> impl Iterator<Item = Point<T>> + Clone {
        (self.y.low..self.y.high)
            .cartesian_product(self.x.low..self.x.high)
            .map(|(y, x)| Point::new(x, y))
    }

    /// All whole number points in [x.low, x.high] x [y.low, y.high]
    pub fn iter_closed(self) -> impl Iterator<Item = Point<T>> + Clone {
        (self.y.low..=self.y.high)
            .cartesian_product(self.x.low..=self.x.high)
            .map(|(y, x)| Point::new(x, y))
    }
}
