use crate::{
    math::{
        arrow::Arrow,
        axis_line::AxisLine,
        generic::{ConstZero, CwiseMul, IntoLossy, Num},
        interval::Interval,
        point::Point,
    },
    utils::ReflectEnum,
};
use itertools::Itertools;
use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    ops::{Add, Mul, Range, Sub},
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

impl<T> Rect<T> {
    pub const fn new(x: Interval<T>, y: Interval<T>) -> Self {
        Self { x, y }
    }

    pub fn intervals(x: impl Into<Interval<T>>, y: impl Into<Interval<T>>) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
        }
    }

    pub fn low_high(low: impl Into<Point<T>>, high: impl Into<Point<T>>) -> Self {
        let (low, high) = (low.into(), high.into());
        Self {
            x: Interval::new(low.x, high.x),
            y: Interval::new(low.y, high.y),
        }
    }

    pub fn cwise_into<S>(self) -> Rect<S>
    where
        T: Into<S>,
    {
        Rect {
            x: self.x.cwise_into(),
            y: self.y.cwise_into(),
        }
    }

    pub fn cwise_into_lossy<S>(self) -> Rect<S>
    where
        T: IntoLossy<S>,
    {
        Rect {
            x: self.x.cwise_into_lossy(),
            y: self.y.cwise_into_lossy(),
        }
    }

    pub fn cwise_try_into<S>(self) -> Result<Rect<S>, <T as TryInto<S>>::Error>
    where
        T: TryInto<S>,
    {
        Ok(Rect::new(
            self.x.cwise_try_into()?,
            self.y.cwise_try_into()?,
        ))
    }
}

impl<T> Rect<T>
where
    T: Copy,
{
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

    pub fn corner(&self, corner: RectCorner) -> Point<T> {
        match corner {
            RectCorner::TopLeft => self.top_left(),
            RectCorner::TopRight => self.top_right(),
            RectCorner::BottomLeft => self.bottom_left(),
            RectCorner::BottomRight => self.bottom_right(),
        }
    }

    /// Counter clockwise
    pub fn corners(&self) -> [Point<T>; 4] {
        [
            self.top_left(),
            self.bottom_left(),
            self.bottom_right(),
            self.top_right(),
        ]
    }

    pub fn bottom(&self) -> T {
        self.y.low
    }

    pub fn mut_bottom(&mut self) -> &mut T {
        &mut self.y.low
    }

    pub fn top(&self) -> T {
        self.y.high
    }

    pub fn mut_top(&mut self) -> &mut T {
        &mut self.y.high
    }

    pub fn right(&self) -> T {
        self.x.high
    }

    pub fn mut_right(&mut self) -> &mut T {
        &mut self.x.high
    }

    pub fn left(&self) -> T {
        self.x.low
    }

    pub fn mut_left(&mut self) -> &mut T {
        &mut self.x.low
    }

    pub fn mut_side(&mut self, side: RectSide) -> &mut T {
        match side {
            RectSide::Left => self.mut_left(),
            RectSide::Right => self.mut_right(),
            RectSide::Bottom => self.mut_bottom(),
            RectSide::Top => self.mut_top(),
        }
    }

    pub fn point(p: Point<T>) -> Self {
        Self::new(Interval::point(p.x), Interval::point(p.y))
    }

    pub fn left_line(&self) -> AxisLine<T> {
        AxisLine::vertical(self.x.low, self.y)
    }

    pub fn right_line(&self) -> AxisLine<T> {
        AxisLine::vertical(self.x.high, self.y)
    }

    pub fn top_line(&self) -> AxisLine<T> {
        AxisLine::horizontal(self.x, self.y.low)
    }

    pub fn bottom_line(&self) -> AxisLine<T> {
        AxisLine::horizontal(self.x, self.y.high)
    }

    pub const TRIANGLE_INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];
}

impl<T: Num> Rect<T> {
    const EMPTY: Self = Self::new(Interval::<T>::EMPTY, Interval::<T>::EMPTY);
    const UNIT: Self = Self::new(Interval::<T>::UNIT, Interval::<T>::UNIT);

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

    pub fn intersects_axis_line(self, rhs: AxisLine<T>) -> bool {
        match rhs {
            AxisLine::Vertical { x, y } => self.x.contains(x) && self.y.intersects(y),
            AxisLine::Horizontal { x, y } => self.x.intersects(x) && self.y.contains(y),
        }
    }

    pub fn interior_intersects_axis_line(self, rhs: AxisLine<T>) -> bool {
        match rhs {
            AxisLine::Vertical { x, y } => {
                self.x.interior_contains(x) && self.y.interior_intersects(y)
            }
            AxisLine::Horizontal { x, y } => {
                self.x.interior_intersects(x) && self.x.interior_contains(y)
            }
        }
    }

    /// interior(lhs) intersects closed(rhs) iff interior(lhs) intersects interior(rhs)
    pub fn interior_intersects(self, rhs: Self) -> bool {
        self.x.interior_intersects(rhs.x) && self.y.interior_intersects(rhs.y)
    }

    pub fn boundary_contains_point(&self, p: Point<T>) -> bool {
        self.left_line().contains_point(p)
            || self.right_line().contains_point(p)
            || self.top_line().contains_point(p)
            || self.bottom_line().contains_point(p)
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

    pub fn ccw_side_arrow(self, side: RectSide) -> Arrow<T> {
        match side {
            RectSide::Left => self.ccw_left_arrow(),
            RectSide::Right => self.ccw_right_arrow(),
            RectSide::Bottom => self.ccw_bottom_arrow(),
            RectSide::Top => self.ccw_top_arrow(),
        }
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
    pub fn low_size(low: impl Into<Point<T>>, size: impl Into<Point<T>>) -> Self {
        let low = low.into();
        let size = size.into();
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

    pub fn padded(self, padding: T) -> Self {
        Self::new(self.x.padded(padding), self.y.padded(padding))
    }

    /// Add one cwise to high, useful for indexing half open / closed rectangles
    pub fn inc_high(self) -> Self {
        Self::new(self.x.inc_high(), self.y.inc_high())
    }

    pub fn is_corner(self, corner: Point<T>) -> bool {
        // TODO: Write a faster method: corner.real in {low.real, high.real} ...
        self.corners().contains(&corner)
    }

    pub fn contains_point(self, p: Point<T>) -> bool {
        self.x.contains(p.x) && self.y.contains(p.y)
    }

    pub fn interior_contains_point(self, p: Point<T>) -> bool {
        self.x.interior_contains(p.x) && self.y.interior_contains(p.y)
    }

    pub fn contains_rect(self, rect: Self) -> bool {
        self.x.contains_interval(rect.x) && self.y.contains_interval(rect.y)
    }

    pub fn interior_contains_rect(self, rect: Self) -> bool {
        self.x.interior_contains_interval(rect.x) && self.y.interior_contains_interval(rect.y)
    }

    pub fn contains_line_segment(self, line: Arrow<T>) -> bool {
        self.contains_point(line.a) && self.contains_point(line.b)
    }

    pub fn interior_contains_line_segment(self, line: Arrow<T>) -> bool {
        self.interior_contains_point(line.a) && self.interior_contains_point(line.b)
    }

    /// Keep bottom left fixed
    pub fn move_top_right_corner(self, top_right_corner: Point<T>) -> Self {
        assert!(!self.is_empty());
        [self.bottom_left(), top_right_corner].bounds()
    }

    /// Keep bottom right fixed
    pub fn move_top_left_corner(self, top_left_corner: Point<T>) -> Self {
        assert!(!self.is_empty());
        [self.bottom_right(), top_left_corner].bounds()

        // Alternative: If top left corner is moved below or to the right of the
        // top right corner, returns an empty QiRect.
        // Self::from_sides(top_left.real, self.right(), self.bottom(), top_left.imag)
    }

    /// Keep top right fixed
    pub fn move_bottom_left_corner(self, bottom_left_corner: Point<T>) -> Self {
        assert!(!self.is_empty());
        [self.top_right(), bottom_left_corner].bounds()
    }

    /// Keep top left fixed
    pub fn move_bottom_right_corner(self, bottom_right_corner: Point<T>) -> Self {
        assert!(!self.is_empty());
        [self.top_left(), bottom_right_corner].bounds()
    }

    pub fn move_corner(self, corner: RectCorner, location: Point<T>) -> Self {
        match corner {
            RectCorner::TopLeft => self.move_top_left_corner(location),
            RectCorner::TopRight => self.move_top_right_corner(location),
            RectCorner::BottomLeft => self.move_bottom_left_corner(location),
            RectCorner::BottomRight => self.move_bottom_right_corner(location),
        }
    }

    pub fn distance_squared(self, p: Point<T>) -> T {
        self.clamp(p).distance_squared(p)
    }

    pub fn top_center(self) -> Point<T> {
        Point::new(self.x.center(), self.y.low)
    }

    pub fn bottom_center(self) -> Point<T> {
        Point::new(self.x.center(), self.y.high)
    }

    pub fn left_center(self) -> Point<T> {
        Point::new(self.x.low, self.y.center())
    }

    pub fn right_center(self) -> Point<T> {
        Point::new(self.x.high, self.y.center())
    }

    pub fn side_center(self, side: RectSide) -> Point<T> {
        match side {
            RectSide::Bottom => self.bottom_center(),
            RectSide::Top => self.top_center(),
            RectSide::Left => self.left_center(),
            RectSide::Right => self.right_center(),
        }
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

/// Right hand component wise mul with Point, panics if rhs.x <= or rhs.y <= 0
impl<T> CwiseMul<Point<T>> for Rect<T>
where
    T: Mul<Output = T> + Copy + PartialOrd + ConstZero,
{
    type Output = Rect<T>;

    fn cwise_mul(self, rhs: Point<T>) -> Self::Output {
        Self::Output::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl<T: Num> PartialEq for Rect<T> {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl<T: Num> Eq for Rect<T> {}

impl<T: Num + Hash> Hash for Rect<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.hash(state);
        self.y.hash(state);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RectCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl RectCorner {
    pub const ALL: [Self; 4] = [
        Self::TopLeft,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomRight,
    ];
}

impl ReflectEnum for RectCorner {
    fn all() -> &'static [Self] {
        &Self::ALL
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::TopLeft => "TopLeft",
            Self::TopRight => "TopRight",
            Self::BottomLeft => "BottomLeft",
            Self::BottomRight => "BottomRight",
        }
    }
}

/// Bitmap rectangle zero is at top left
///    top left    top right
///      0,0         1,0
///       ┌───────────┐
///       │           │
///       └───────────┘
///      0,1         1,1
/// bottom left   bottom right
const fn unit_bitmap_corner(corner: RectCorner) -> Point<i64> {
    match corner {
        RectCorner::TopLeft => Point::new(0, 0),
        RectCorner::BottomLeft => Point::new(0, 1),
        RectCorner::BottomRight => Point::new(1, 1),
        RectCorner::TopRight => Point::new(1, 0),
    }
}

/// OpenGL UV rectangle
///    top left    top right
///      0,1         1,1
///       ┌───────────┐
///       │           │
///       └───────────┘
///      0,0         1,0
/// bottom left   bottom right
const fn uv_corner(corner: RectCorner) -> Point<i64> {
    match corner {
        RectCorner::TopLeft => Point::new(0, 1),
        RectCorner::BottomLeft => Point::new(0, 0),
        RectCorner::BottomRight => Point::new(1, 0),
        RectCorner::TopRight => Point::new(1, 1),
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RectSide {
    Left,
    Right,
    Bottom,
    Top,
}

impl RectSide {
    pub const ALL: [Self; 4] = [Self::Left, Self::Right, Self::Bottom, Self::Top];
}

impl ReflectEnum for RectSide {
    fn all() -> &'static [Self] {
        &Self::ALL
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Bottom => "Bottom",
            Self::Top => "Top",
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

/// Convert to egui
impl From<Rect<f32>> for egui::Rect {
    fn from(value: Rect<f32>) -> Self {
        Self::from_min_max(value.low().into(), value.high().into())
    }
}

impl From<egui::Rect> for Rect<f32> {
    fn from(value: egui::Rect) -> Self {
        let low: Point<f32> = value.min.into();
        let high: Point<f32> = value.max.into();
        Self::low_high(low, high)
    }
}

/// Iterators on Rect<usize>, could be extended to any integer types
impl<T> Rect<T>
where
    T: Clone,
    Range<T>: Clone + Iterator<Item = T>,
{
    pub fn iter_half_open(self) -> impl Iterator<Item = Point<T>> + Clone {
        (self.y.low..self.y.high)
            .cartesian_product(self.x.low..self.x.high)
            .map(|(y, x)| Point::new(x, y))
    }
}

#[cfg(test)]
mod test {
    use crate::math::rect::Rect;

    #[test]
    fn test_boundary_contains_point() {
        let rect = Rect::<i64>::intervals([0, 2], [0, 2]);
        // Corners
        assert!(rect.boundary_contains_point([0, 0].into()));
        assert!(rect.boundary_contains_point([2, 0].into()));
        assert!(rect.boundary_contains_point([0, 2].into()));
        assert!(rect.boundary_contains_point([2, 2].into()));
        // Sides
        assert!(rect.boundary_contains_point([0, 1].into()));
        assert!(rect.boundary_contains_point([1, 0].into()));
        assert!(rect.boundary_contains_point([2, 1].into()));
        assert!(rect.boundary_contains_point([1, 2].into()));
    }
}
