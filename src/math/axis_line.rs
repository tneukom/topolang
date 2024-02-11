use super::{interval::Interval, point::Point};
use crate::{
    math::generic::{CwiseMul, Num},
    utils::IteratorPlus,
};
use itertools::Either;
use std::ops::{Add, Mul, Sub};

/// Similar to Range
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span<T> {
    pub start: T,
    pub stop: T,
}

impl<T> Span<T> {
    pub const fn new(start: T, stop: T) -> Self {
        Self { start, stop }
    }

    pub fn cwise_into<S>(self) -> Span<S>
    where
        T: Into<S>,
    {
        Span {
            start: self.start.into(),
            stop: self.stop.into(),
        }
    }
}

impl<T: Num> Span<T> {
    pub fn to_interval(self) -> Interval<T> {
        if self.start < self.stop {
            Interval::new(self.start, self.stop)
        } else {
            Interval::new(self.stop, self.start)
        }
    }

    pub fn is_increasing(self) -> bool {
        self.start < self.stop
    }

    pub fn is_decreasing(self) -> bool {
        self.start > self.stop
    }

    pub fn contains(self, value: T) -> bool {
        self.to_interval().contains(value)
    }
}

impl Span<i64> {
    /// iterator that yields start, start + 1, ..., stop - 1 if stop > start
    /// and start, start - 1, ..., stop + 1
    fn steps_excluding_stop(self) -> impl IteratorPlus<i64> {
        if self.start < self.stop {
            Either::Left(self.start..self.stop)
        } else {
            Either::Right(((self.stop + 1)..=self.start).rev())
        }
    }
}

impl<S, T: From<S>> From<[S; 2]> for Span<T> {
    fn from(value: [S; 2]) -> Self {
        let [x, y] = value;
        Self::new(x.into(), y.into())
    }
}

/// Right multiplication because Rust cannot handle generic left multiplication
impl<T> Mul<T> for Span<T>
where
    T: Mul<Output = T> + Copy,
{
    type Output = Span<T>;

    fn mul(self, rhs: T) -> Self::Output {
        Span {
            start: self.start * rhs,
            stop: self.stop * rhs,
        }
    }
}

impl<T> Add<T> for Span<T>
where
    T: Add<Output = T> + Copy,
{
    type Output = Span<T>;

    fn add(self, rhs: T) -> Self::Output {
        Span {
            start: self.start + rhs,
            stop: self.stop + rhs,
        }
    }
}

impl<T> Sub<T> for Span<T>
where
    T: Sub<Output = T> + Copy,
{
    type Output = Span<T>;

    fn sub(self, rhs: T) -> Self::Output {
        Span {
            start: self.start - rhs,
            stop: self.stop - rhs,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisArrow<T> {
    Vertical { x: T, y: Span<T> },
    Horizontal { x: Span<T>, y: T },
}

impl<T> AxisArrow<T> {
    pub fn axis(self) -> Axis {
        match self {
            Self::Vertical { .. } => Axis::Vertical,
            Self::Horizontal { .. } => Axis::Horizontal,
        }
    }

    pub fn fixed(self) -> T {
        match self {
            Self::Vertical { x, .. } => x,
            Self::Horizontal { y, .. } => y,
        }
    }

    pub fn varying(self) -> Span<T> {
        match self {
            Self::Vertical { y, .. } => y,
            Self::Horizontal { x, .. } => x,
        }
    }

    pub fn cwise_into<S>(self) -> AxisArrow<S>
    where
        T: Into<S>,
    {
        match self {
            Self::Vertical { x, y } => AxisArrow::Vertical {
                x: x.into(),
                y: y.cwise_into(),
            },
            Self::Horizontal { x, y } => AxisArrow::Horizontal {
                x: x.cwise_into(),
                y: y.into(),
            },
        }
    }

    pub const fn vertical(x: T, y: Span<T>) -> Self {
        Self::Vertical { x, y }
    }

    pub fn vertical_from(x: impl Into<T>, y: impl Into<Span<T>>) -> Self {
        Self::vertical(x.into(), y.into())
    }

    pub const fn horizontal(x: Span<T>, y: T) -> Self {
        Self::Horizontal { x, y }
    }

    pub fn horizontal_from(x: Span<T>, y: T) -> Self {
        Self::horizontal(x.into(), y.into())
    }

    pub fn start_point(self) -> Point<T> {
        match self {
            Self::Vertical { x, y } => Point::new(x, y.start),
            Self::Horizontal { x, y } => Point::new(x.start, y),
        }
    }

    pub fn stop_point(self) -> Point<T> {
        match self {
            Self::Vertical { x, y } => Point::new(x, y.stop),
            Self::Horizontal { x, y } => Point::new(x.stop, y),
        }
    }
}

impl<T: Num> AxisArrow<T> {
    pub fn to_line(self) -> AxisLine<T> {
        match self {
            Self::Vertical { x, y } => AxisLine::Vertical {
                x,
                y: y.to_interval(),
            },
            Self::Horizontal { x, y } => AxisLine::Horizontal {
                x: x.to_interval(),
                y,
            },
        }
    }

    pub fn from_points(start: Point<T>, stop: Point<T>) -> Option<Self> {
        if start.x == stop.x {
            Some(Self::vertical(start.x, Span::new(start.y, stop.y)))
        } else if start.y == stop.y {
            Some(Self::horizontal(Span::new(start.x, stop.x), start.y))
        } else {
            None
        }
    }

    pub fn contains_point(self, point: Point<T>) -> bool {
        match self {
            Self::Vertical { x, y } => x == point.x && y.contains(point.y),
            Self::Horizontal { x, y } => x.contains(point.x) && y == point.y,
        }
    }
}

impl<T> CwiseMul<Point<T>> for AxisArrow<T>
where
    T: Mul<Output = T> + Copy,
{
    type Output = AxisArrow<T>;

    fn cwise_mul(self, rhs: Point<T>) -> Self::Output {
        match self {
            Self::Vertical { x, y } => Self::Output::Vertical {
                x: x * rhs.x,
                y: y * rhs.y,
            },
            Self::Horizontal { x, y } => Self::Output::Horizontal {
                x: x * rhs.x,
                y: y * rhs.y,
            },
        }
    }
}

impl<T> Add<Point<T>> for AxisArrow<T>
where
    T: Add<Output = T> + Copy,
{
    type Output = AxisArrow<T>;

    fn add(self, rhs: Point<T>) -> Self::Output {
        match self {
            Self::Vertical { x, y } => Self::Output::Vertical {
                x: x + rhs.x,
                y: y + rhs.y,
            },
            Self::Horizontal { x, y } => Self::Output::Horizontal {
                x: x + rhs.x,
                y: y + rhs.y,
            },
        }
    }
}

impl AxisArrow<i64> {
    /// Includes start but not stop
    pub fn steps_excluding_stop(self) -> impl IteratorPlus<Point<i64>> {
        match self {
            Self::Vertical { x, y } => {
                Either::Left(y.steps_excluding_stop().map(move |y| Point::new(x, y)))
            }
            Self::Horizontal { x, y } => {
                Either::Right(x.steps_excluding_stop().map(move |x| Point::new(x, y)))
            }
        }
    }
}

pub enum AxisLine<T> {
    Vertical { x: T, y: Interval<T> },
    Horizontal { x: Interval<T>, y: T },
}

impl<T> AxisLine<T> {
    pub fn axis(self) -> Axis {
        match self {
            Self::Vertical { .. } => Axis::Vertical,
            Self::Horizontal { .. } => Axis::Horizontal,
        }
    }

    pub fn fixed(self) -> T {
        match self {
            Self::Vertical { x, .. } => x,
            Self::Horizontal { y, .. } => y,
        }
    }

    pub fn varying(self) -> Interval<T> {
        match self {
            Self::Vertical { y, .. } => y,
            Self::Horizontal { x, .. } => x,
        }
    }

    pub const fn vertical(x: T, y: Interval<T>) -> Self {
        Self::Vertical { x, y }
    }

    pub fn vertical_from(x: impl Into<T>, y: impl Into<Interval<T>>) -> Self {
        Self::vertical(x.into(), y.into())
    }

    pub const fn horizontal(x: Interval<T>, y: T) -> Self {
        Self::Horizontal { x, y }
    }

    pub fn horizontal_from(x: Interval<T>, y: T) -> Self {
        Self::horizontal(x.into(), y.into())
    }
}

impl<T: Num> AxisLine<T> {
    pub fn contains_point(self, p: Point<T>) -> bool {
        match self {
            Self::Vertical { x, y } => x == p.x && y.contains(p.y),
            Self::Horizontal { x, y } => x.contains(p.x) && y == p.y,
        }
    }

    pub fn nearest(self, p: Point<T>) -> Point<T> {
        match self {
            Self::Vertical { x, y } => Point::new(x, y.clamp(p.y)),
            Self::Horizontal { x, y } => Point::new(x.clamp(p.x), y),
        }
    }

    pub fn sup_distance(self, p: Point<T>) -> T {
        let distance_delta = p - self.nearest(p);
        distance_delta.x.max(distance_delta.y)
    }

    pub fn length(self) -> T {
        self.varying().length()
    }

    pub fn distance_squared(self, p: Point<T>) -> T {
        let distance_delta = p - self.nearest(p);
        distance_delta.x * distance_delta.x + distance_delta.y * distance_delta.y
    }
}
