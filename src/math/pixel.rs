use std::{
    fmt::{Debug, Display, Formatter},
    ops::Add,
};

/// See pixel_pattern.jpg, sides_and_corners.jpg
use crate::math::point::Point;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SideName {
    Left,
    BottomLeft,
    Bottom,
    Right,
    TopRight,
    Top,
}

impl SideName {
    pub const fn opposite(self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Left => Self::Right,
            Self::BottomLeft => Self::TopRight,
            Self::Bottom => Self::Top,
            Self::Right => Self::Left,
            Self::TopRight => Self::BottomLeft,
        }
    }

    pub const fn next_ccw(self) -> Self {
        match self {
            Self::Top => Self::Left,
            Self::Left => Self::BottomLeft,
            Self::BottomLeft => Self::Bottom,
            Self::Bottom => Self::Right,
            Self::Right => Self::TopRight,
            Self::TopRight => Self::Top,
        }
    }

    pub const fn previous_ccw(self) -> Self {
        match self {
            Self::Top => Self::TopRight,
            Self::Left => Self::Top,
            Self::BottomLeft => Self::Left,
            Self::Bottom => Self::BottomLeft,
            Self::Right => Self::Bottom,
            Self::TopRight => Self::Right,
        }
    }

    pub fn unicode_symbol(self) -> char {
        match self {
            SideName::Top => '←',
            SideName::Left => '↓',
            SideName::BottomLeft => '↘',
            SideName::Bottom => '→',
            SideName::Right => '↑',
            SideName::TopRight => '↖',
        }
    }

    pub const ALL: [SideName; 6] = [
        Self::Top,
        Self::Left,
        Self::BottomLeft,
        Self::Bottom,
        Self::Right,
        Self::TopRight,
    ];
}

/// Each pixel is connected to its top, left, bottom, right and bottom-left, top-right neighbors,
/// see docs/pixel_pattern.jpg and
impl Point<i64> {
    pub fn containing(p: Point<f64>) -> Self {
        let i64_p = p.floor().cwise_into_lossy::<i64>();
        Self::new(i64_p.x, i64_p.y)
    }

    pub const fn top_neighbor(self) -> Self {
        Self::new(self.x, self.y - 1)
    }

    pub const fn left_neighbor(self) -> Self {
        Self::new(self.x - 1, self.y)
    }

    pub const fn bottom_left_neighbor(self) -> Self {
        Self::new(self.x - 1, self.y + 1)
    }

    pub const fn bottom_neighbor(self) -> Self {
        Self::new(self.x, self.y + 1)
    }

    pub const fn right_neighbor(self) -> Self {
        Self::new(self.x + 1, self.y)
    }

    pub const fn top_right_neighbor(self) -> Self {
        Self::new(self.x + 1, self.y - 1)
    }

    pub const fn neighbor(self, side: SideName) -> Self {
        match side {
            SideName::Top => self.top_neighbor(),
            SideName::Left => self.left_neighbor(),
            SideName::BottomLeft => self.bottom_left_neighbor(),
            SideName::Bottom => self.bottom_neighbor(),
            SideName::Right => self.right_neighbor(),
            SideName::TopRight => self.top_right_neighbor(),
        }
    }

    pub const fn neighbors(self) -> [Self; 6] {
        [
            self.top_neighbor(),
            self.left_neighbor(),
            self.bottom_left_neighbor(),
            self.bottom_neighbor(),
            self.right_neighbor(),
            self.top_right_neighbor(),
        ]
    }

    pub const fn top_side(self) -> Side {
        Side::new(self, SideName::Top)
    }

    pub const fn left_side(self) -> Side {
        Side::new(self, SideName::Left)
    }

    pub const fn bottom_left_side(self) -> Side {
        Side::new(self, SideName::BottomLeft)
    }

    pub const fn bottom_side(self) -> Side {
        Side::new(self, SideName::Bottom)
    }

    pub const fn right_side(self) -> Side {
        Side::new(self, SideName::Right)
    }

    pub const fn top_right_side(self) -> Side {
        Side::new(self, SideName::TopRight)
    }

    pub const fn side_ccw(self, side_name: SideName) -> Side {
        Side::new(self, side_name)
    }

    pub const fn sides_ccw(self) -> [Side; 6] {
        [
            self.top_side(),
            self.left_side(),
            self.bottom_left_side(),
            self.bottom_side(),
            self.right_side(),
            self.top_right_side(),
        ]
    }

    pub fn sides_cw(self) -> [Side; 6] {
        self.sides_ccw().map(|side| side.reversed())
    }

    pub fn corners(self) -> [Corner; 6] {
        self.sides_ccw().map(|side| side.start_corner())
    }

    /// self is a neighbor of other lhs.is_neighbor(rhs) <=> rhs.is_neighbor(lhs)
    pub fn is_neighbor(self, other: Self) -> bool {
        self.neighbors().iter().any(|&neighbor| neighbor == other)
    }

    pub fn touches(self, other: Self) -> bool {
        self == other || self.is_neighbor(other)
    }

    /// self + neighbors
    pub fn touching(self) -> [Self; 7] {
        // TODO: Would be better if there was an array concat
        [
            self,
            self.top_neighbor(),
            self.left_neighbor(),
            self.bottom_left_neighbor(),
            self.bottom_neighbor(),
            self.right_neighbor(),
            self.top_right_neighbor(),
        ]
    }
}

pub type Pixel = Point<i64>;

/// Side(pixel, side) is the counterclockwise side around pixel
/// Each pixel has therefore 6 sides, see docs/sides_and_corners.jpg
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Side {
    /// pixel on the left side of the side
    pub left_pixel: Pixel,
    pub name: SideName,
}

impl Side {
    pub const fn new(pixel: Pixel, side_name: SideName) -> Self {
        Self {
            left_pixel: pixel,
            name: side_name,
        }
    }

    /// Same side but with reversed orientation, cw -> ccw, ccw -> cw
    pub const fn reversed(self) -> Self {
        Self::new(self.left_pixel.neighbor(self.name), self.name.opposite())
    }

    pub const fn next_ccw(self) -> Self {
        Self::new(self.left_pixel, self.name.next_ccw())
    }

    pub const fn previous_ccw(self) -> Self {
        Self::new(self.left_pixel, self.name.previous_ccw())
    }

    pub const fn start_corner(self) -> Corner {
        self.reversed().stop_corner()
    }

    pub const fn stop_corner(self) -> Corner {
        Corner::side_stop(self)
    }

    pub fn left_pixel(self) -> Pixel {
        self.left_pixel
    }

    pub fn right_pixel(self) -> Pixel {
        self.reversed().left_pixel
    }

    pub fn continuing_sides(self) -> [Side; 2] {
        [self.next_ccw(), self.reversed().previous_ccw().reversed()]
    }
}

impl Add<Point<i64>> for Side {
    type Output = Side;

    fn add(self, rhs: Point<i64>) -> Self::Output {
        Self::new(self.left_pixel + rhs, self.name)
    }
}

impl Display for Side {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Side({}, {})",
            self.left_pixel,
            self.name.unicode_symbol()
        )
    }
}

impl Debug for Side {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CornerName {
    /// Start corner of top side
    TopStart,

    /// Stop corner of top side
    TopStop,
}

/// Corner(side: Side) is the stop corner of side
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Corner {
    pub pixel: Pixel,
    pub name: CornerName,
}

impl Corner {
    pub const fn new(pixel: Pixel, name: CornerName) -> Self {
        Self { pixel, name }
    }

    /// Stop corner of top right side
    pub const fn top_right_side_stop(pixel: Pixel) -> Self {
        Self::new(pixel, CornerName::TopStart)
    }

    /// Stop corner of top side
    pub const fn top_side_stop(pixel: Pixel) -> Self {
        Self::new(pixel, CornerName::TopStop)
    }

    /// Stop corner of left side
    pub const fn left_side_stop(pixel: Pixel) -> Self {
        Self::new(pixel.bottom_left_neighbor(), CornerName::TopStart)
    }

    /// Stop corner of bottom left side
    pub const fn bottom_left_side_stop(pixel: Pixel) -> Self {
        Self::new(pixel.bottom_neighbor(), CornerName::TopStop)
    }

    /// Stop corner of bottom side
    pub const fn bottom_side_stop(pixel: Pixel) -> Self {
        Self::new(pixel.bottom_neighbor(), CornerName::TopStart)
    }

    /// Stop corner of right side
    pub const fn right_side_stop(pixel: Pixel) -> Self {
        Self::new(pixel.right_neighbor(), CornerName::TopStop)
    }

    /// See docs/corner_normalization.png
    pub const fn side_stop(side: Side) -> Self {
        match side.name {
            SideName::Top => Self::top_side_stop(side.left_pixel),
            SideName::Left => Self::left_side_stop(side.left_pixel),
            SideName::BottomLeft => Self::bottom_left_side_stop(side.left_pixel),
            SideName::Bottom => Self::bottom_side_stop(side.left_pixel),
            SideName::Right => Self::right_side_stop(side.left_pixel),
            SideName::TopRight => Self::top_right_side_stop(side.left_pixel),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::math::pixel::{Pixel, Side, SideName};

    const PIXELS: [Pixel; 3] = [Pixel::new(0, 0), Pixel::new(-2, 4), Pixel::new(9, 17)];

    fn example_sides() -> Vec<Side> {
        PIXELS
            .into_iter()
            .flat_map(|pixel| pixel.sides_ccw())
            .collect()
    }

    #[test]
    fn opposite_of_opposite() {
        for side_name in SideName::ALL {
            assert_eq!(side_name.opposite().opposite(), side_name);
        }
    }

    #[test]
    fn previous_of_next() {
        for side in example_sides() {
            assert_eq!(side, side.next_ccw().previous_ccw());
            assert_eq!(side, side.previous_ccw().next_ccw());
        }
    }

    #[test]
    fn opposite_neighbor_pixel() {
        for side_name in SideName::ALL {
            for pixel in PIXELS {
                assert_eq!(
                    pixel.neighbor(side_name).neighbor(side_name.opposite()),
                    pixel
                );
            }
        }
    }

    /// Make sure stop(side) == start(side.next_ccw())
    #[test]
    fn start_stop_corner() {
        for side in example_sides() {
            assert_eq!(side.stop_corner(), side.next_ccw().start_corner())
        }
    }

    #[test]
    fn equivalent_corners() {
        for pixel in PIXELS {
            // top side
            assert_eq!(
                pixel.top_side().stop_corner(),
                pixel.left_neighbor().right_side().stop_corner()
            );
            assert_eq!(
                pixel.top_side().stop_corner(),
                pixel.top_neighbor().bottom_left_side().stop_corner()
            );

            // left side
            assert_eq!(
                pixel.left_side().stop_corner(),
                pixel.left_neighbor().bottom_side().stop_corner()
            );
            assert_eq!(
                pixel.left_side().stop_corner(),
                pixel.bottom_left_neighbor().top_right_side().stop_corner()
            );

            // bottom left side
            assert_eq!(
                pixel.bottom_left_side().stop_corner(),
                pixel.bottom_neighbor().top_side().stop_corner()
            );
            assert_eq!(
                pixel.bottom_left_side().stop_corner(),
                pixel.bottom_left_neighbor().right_side().stop_corner()
            );

            // bottom side
            assert_eq!(
                pixel.bottom_side().stop_corner(),
                pixel.right_neighbor().left_side().stop_corner()
            );
            assert_eq!(
                pixel.bottom_side().stop_corner(),
                pixel.bottom_neighbor().top_right_side().stop_corner()
            );

            // right side
            assert_eq!(
                pixel.right_side().stop_corner(),
                pixel.right_neighbor().top_side().stop_corner()
            );
            assert_eq!(
                pixel.right_side().stop_corner(),
                pixel.top_right_neighbor().bottom_left_side().stop_corner()
            );

            // top right side
            assert_eq!(
                pixel.top_right_side().stop_corner(),
                pixel.top_neighbor().bottom_side().stop_corner()
            );
            assert_eq!(
                pixel.top_right_side().stop_corner(),
                pixel.top_right_neighbor().left_side().stop_corner()
            );
        }
    }

    // TODO: Make static assert if possible
    #[test]
    fn side_ordering() {
        let pixel = Pixel::new(0, 0);

        let ccw_sides = pixel.sides_ccw();
        assert!(ccw_sides.iter().all(|&side| pixel.left_side() <= side));

        let cw_sides = pixel.sides_cw();
        assert!(cw_sides
            .iter()
            .all(|&side| pixel.top_side().reversed() <= side));
    }
}
