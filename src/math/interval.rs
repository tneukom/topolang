use std::{
    hash::{Hash, Hasher},
    ops::{Add, Mul, Range, RangeInclusive, Sub},
};

use crate::math::generic::{Cast, ConstZero, Num};

#[derive(Clone, Copy, Debug)]
pub struct Interval<T> {
    pub low: T,
    pub high: T,
}

impl<T> Interval<T> {
    // Intervals where low < high are empty
    pub const fn new(low: T, high: T) -> Self {
        Self { low, high }
    }

    pub fn new_from(low: impl Into<T>, high: impl Into<T>) -> Self {
        Self {
            low: low.into(),
            high: high.into(),
        }
    }

    pub fn cwise_into<S>(self) -> Interval<S>
    where
        T: Into<S>,
    {
        Interval {
            low: self.low.into(),
            high: self.high.into(),
        }
    }

    pub fn cwise_cast<S>(self) -> Interval<S>
    where
        T: Cast<S>,
    {
        Interval {
            low: self.low.cast(),
            high: self.high.cast(),
        }
    }

    pub fn cwise_try_into<S>(self) -> Result<Interval<S>, <T as TryInto<S>>::Error>
    where
        T: TryInto<S>,
    {
        Ok(Interval {
            low: self.low.try_into()?,
            high: self.high.try_into()?,
        })
    }
}

impl<T> Interval<T>
where
    T: Copy,
{
    pub fn point(value: T) -> Self {
        Self::new(value.clone(), value)
    }
}

impl<T: Num> Interval<T> {
    pub fn is_empty(&self) -> bool {
        self.low > self.high
    }

    pub fn is_point(&self) -> bool {
        self.low == self.high
    }

    pub fn has_zero_length(self) -> bool {
        self.high <= self.low
    }

    /// value in [low, high]
    pub fn contains(self, value: T) -> bool {
        self.low <= value && value <= self.high
    }

    /// value in [low, high)
    pub fn half_open_contains(self, value: T) -> bool {
        self.low <= value && value < self.high
    }

    pub fn clamp(self, value: T) -> T {
        assert!(!self.is_empty());
        if value < self.low {
            self.low
        } else if value > self.high {
            self.high
        } else {
            value
        }
    }

    pub fn intersect(self, rhs: Self) -> Self {
        Self {
            low: self.low.max(rhs.low),
            high: self.high.min(rhs.high),
        }
    }

    pub fn intersects(self, rhs: Self) -> bool {
        self.high.min(rhs.high) >= self.low.max(rhs.low)
    }

    /// interior(lhs) intersects closed(rhs) iff interior(lhs) intersects interior(rhs)
    pub fn interior_intersects(self, rhs: Self) -> bool {
        self.high.min(rhs.high) > self.low.max(rhs.low)
    }

    pub fn bounds_with_interval(self, rhs: Self) -> Self {
        // Without these two special cases the following case would fail as an example:
        // Bounds(Empty, [10, 11]) = Bounds([1, -1], [10, 11]) = [1, 11])
        // An empty interval is not necessarily [inf, -inf]
        if self.is_empty() {
            return rhs;
        }
        if rhs.is_empty() {
            return self;
        }

        let low = self.low.min(rhs.low);
        let high = self.high.max(rhs.high);
        Self::new(low, high)
    }

    pub fn bounds_with_value(self, rhs: T) -> Self {
        self.bounds_with_interval(Self::point(rhs))
    }

    pub fn distance(self, value: T) -> T {
        assert!(!self.is_empty());
        if value < self.low {
            self.low - value
        } else if value > self.high {
            value - self.high
        } else {
            T::ZERO
        }
    }

    pub fn length(self) -> T {
        if self.is_empty() {
            T::ZERO
        } else {
            self.high - self.low
        }
    }

    pub fn padded(self, padding: T) -> Self {
        assert!(!self.is_empty());
        return Self::new(self.low - padding, self.high + padding);
    }

    pub fn inc_high(self) -> Self {
        return Self::new(self.low, self.high + T::ONE);
    }

    // Must be true:
    //  Empty `union` I = I
    //  Empty `intersect` I = Empty
    pub const UNIT: Self = Self::new(T::ZERO, T::ONE);
    pub const EMPTY: Self = Self::new(T::ONE, T::ZERO);

    pub fn interior_contains(self, r: T) -> bool {
        self.low < r && r < self.high
    }

    pub fn contains_interval(self, other: Self) -> bool {
        if other.is_empty() {
            return true;
        }
        self.low <= other.low && other.high <= self.high
    }

    pub fn interior_contains_interval(self, other: Self) -> bool {
        if other.is_empty() {
            return true;
        }
        self.low < other.low && other.high < self.high
    }

    pub fn center(self) -> T {
        assert!(!self.is_empty());
        (self.high + self.low) / T::TWO
    }
}

impl<T> Interval<T>
where
    T: Clone,
    Range<T>: Clone + Iterator<Item = T>,
    RangeInclusive<T>: Clone + Iterator<Item = T>,
{
    /// All whole number points in [low, high)
    pub fn iter_half_open(self) -> impl Iterator<Item = T> + Clone {
        self.low..self.high
    }

    /// All whole number points in [low, high]
    pub fn iter_closed(self) -> impl Iterator<Item = T> + Clone {
        self.low..=self.high
    }
}

impl<T> From<[T; 2]> for Interval<T> {
    fn from(value: [T; 2]) -> Self {
        let [x, y] = value;
        Self::new(x, y)
    }
}

impl Interval<i64> {
    pub const ALL: Self = Self::new(i64::MIN, i64::MAX);
}

impl Interval<f64> {
    pub const ALL: Self = Self::new(f64::NEG_INFINITY, f64::INFINITY);
}

/// Right hand scalar add
impl<T> Add<T> for Interval<T>
where
    T: Add<Output = T> + Copy,
{
    type Output = Interval<T>;

    fn add(self, rhs: T) -> Self::Output {
        Self::Output::new(self.low + rhs, self.high + rhs)
    }
}

/// Right hand scalar sub
impl<T> Sub<T> for Interval<T>
where
    T: Sub<Output = T> + Copy,
{
    type Output = Interval<T>;

    fn sub(self, rhs: T) -> Self::Output {
        Self::Output::new(self.low - rhs, self.high - rhs)
    }
}

/// Right hand scalar mul, panics if rhs <= 0
impl<T> Mul<T> for Interval<T>
where
    T: Mul<Output = T> + Copy + PartialOrd + ConstZero,
{
    type Output = Interval<T>;

    fn mul(self, rhs: T) -> Self::Output {
        assert!(rhs > T::ZERO);
        Self::Output::new(self.low * rhs, self.high * rhs)
    }
}

impl<T: Num> PartialEq for Interval<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.is_empty() {
            other.is_empty()
        } else {
            self.low == other.low && self.high == other.high
        }
    }
}

impl<T: Num> Eq for Interval<T> {}

impl<T: Num + Hash> Hash for Interval<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if self.is_empty() {
            // TODO: Is this ok?
            0.hash(state);
        } else {
            self.low.hash(state);
            self.high.hash(state);
        }
    }
}

pub trait IntervalBounds<T: Num>: Sized {
    fn bounds(self) -> Interval<T>;

    fn iter_bounds(iter: impl Iterator<Item = Self>) -> Interval<T> {
        iter.map(Self::bounds)
            .fold(Interval::EMPTY, Interval::bounds_with_interval)
    }
}

impl<T: Num> IntervalBounds<T> for T {
    fn bounds(self) -> Interval<T> {
        Interval::point(self)
    }
}

impl<T: Num, E, const N: usize> IntervalBounds<T> for [E; N]
where
    E: IntervalBounds<T>,
{
    fn bounds(self) -> Interval<T> {
        self.into_iter()
            .map(E::bounds)
            .reduce(Interval::bounds_with_interval)
            .unwrap()
    }
}
