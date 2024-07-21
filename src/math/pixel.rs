/// See pixel_pattern.jpg, sides_and_corners.jpg
use crate::math::point::Point;
use crate::math::rect::Rect;
use std::collections::BTreeSet;
/// The centers of a hexagon grid form a lattice but the corners do not! However, the corners
/// can be split into two sets each of which forms a lattice. Corners are more cumbersome to handle
/// than cells and sides and terminology is less clear.
/// Order on sides is not very intuitive, a more useful order might be by (corner, out direction).
/// Interesting links:
/// https://en.wikipedia.org/wiki/Lattice_(group)
/// https://www.redblobgames.com/grids/hexagons/
use std::{
    fmt::{Debug, Display, Formatter},
    ops::Add,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SideName {
    Left,
    Bottom,
    BottomRight,
    Right,
    Top,
    TopLeft,
}

impl SideName {
    pub const fn opposite(self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::TopLeft => Self::BottomRight,
            Self::Left => Self::Right,
            Self::Bottom => Self::Top,
            Self::BottomRight => Self::TopLeft,
            Self::Right => Self::Left,
        }
    }

    pub const fn next_ccw(self) -> Self {
        match self {
            Self::Top => Self::TopLeft,
            Self::TopLeft => Self::Left,
            Self::Left => Self::Bottom,
            Self::Bottom => Self::BottomRight,
            Self::BottomRight => Self::Right,
            Self::Right => Self::Top,
        }
    }

    pub const fn previous_ccw(self) -> Self {
        match self {
            Self::Top => Self::Right,
            Self::TopLeft => Self::Top,
            Self::Left => Self::TopLeft,
            Self::Bottom => Self::Left,
            Self::BottomRight => Self::Bottom,
            Self::Right => Self::BottomRight,
        }
    }

    pub fn unicode_symbol(self) -> char {
        match self {
            Self::Top => '←',
            Self::TopLeft => '↙',
            Self::Left => '↓',
            Self::Bottom => '→',
            Self::BottomRight => '↗',
            Self::Right => '↑',
        }
    }

    pub const ALL: [SideName; 6] = [
        Self::Left,
        Self::Bottom,
        Self::BottomRight,
        Self::Right,
        Self::Top,
        Self::TopLeft,
    ];
}

/// Each pixel is connected to its top, left, bottom, right and bottom-left, top-right neighbors,
/// see docs/pixel_pattern.jpg and
impl Point<i64> {
    pub fn containing(p: Point<f64>) -> Self {
        let i64_p = p.floor().cwise_cast::<i64>();
        Self::new(i64_p.x, i64_p.y)
    }

    pub const fn top_neighbor(self) -> Self {
        Self::new(self.x, self.y - 1)
    }

    pub const fn top_left_neighbor(self) -> Self {
        Self::new(self.x - 1, self.y - 1)
    }

    pub const fn left_neighbor(self) -> Self {
        Self::new(self.x - 1, self.y)
    }

    pub const fn bottom_neighbor(self) -> Self {
        Self::new(self.x, self.y + 1)
    }

    pub const fn bottom_right_neighbor(self) -> Self {
        Self::new(self.x + 1, self.y + 1)
    }

    pub const fn right_neighbor(self) -> Self {
        Self::new(self.x + 1, self.y)
    }

    pub const fn neighbor(self, side: SideName) -> Self {
        match side {
            SideName::Top => self.top_neighbor(),
            SideName::TopLeft => self.top_left_neighbor(),
            SideName::Left => self.left_neighbor(),
            SideName::Bottom => self.bottom_neighbor(),
            SideName::BottomRight => self.bottom_right_neighbor(),
            SideName::Right => self.right_neighbor(),
        }
    }

    pub const fn neighbors(self) -> [Self; 6] {
        [
            self.top_neighbor(),
            self.top_left_neighbor(),
            self.left_neighbor(),
            self.bottom_neighbor(),
            self.bottom_right_neighbor(),
            self.right_neighbor(),
        ]
    }

    pub const fn top_side(self) -> Side {
        Side::new(self, SideName::Top)
    }

    pub const fn top_left_side(self) -> Side {
        Side::new(self, SideName::TopLeft)
    }

    pub const fn left_side(self) -> Side {
        Side::new(self, SideName::Left)
    }

    pub const fn bottom_side(self) -> Side {
        Side::new(self, SideName::Bottom)
    }

    pub const fn bottom_right_side(self) -> Side {
        Side::new(self, SideName::BottomRight)
    }

    pub const fn right_side(self) -> Side {
        Side::new(self, SideName::Right)
    }

    pub const fn side_ccw(self, side_name: SideName) -> Side {
        Side::new(self, side_name)
    }

    pub const fn sides_ccw(self) -> [Side; 6] {
        [
            self.top_side(),
            self.top_left_side(),
            self.left_side(),
            self.bottom_side(),
            self.bottom_right_side(),
            self.right_side(),
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
            self.top_left_neighbor(),
            self.left_neighbor(),
            self.bottom_neighbor(),
            self.bottom_right_neighbor(),
            self.right_neighbor(),
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

/// Very slow, cache the result!
pub fn rect_boundary_sides(rect: Rect<i64>) -> BTreeSet<Side> {
    rect.iter_half_open()
        .flat_map(Pixel::sides_ccw)
        .filter(|side| !rect.half_open_contains(side.right_pixel()))
        .collect()
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

    /// Stop corner of top side
    pub const fn top_side_stop(pixel: Pixel) -> Self {
        Self::new(pixel, CornerName::TopStop)
    }

    /// Stop corner of the top left side
    pub const fn top_left_side_stop(pixel: Pixel) -> Self {
        Self::new(pixel.left_neighbor(), CornerName::TopStart)
    }

    /// Stop corner of left side
    pub const fn left_side_stop(pixel: Pixel) -> Self {
        Self::new(pixel.bottom_neighbor(), CornerName::TopStop)
    }

    /// Stop corner of bottom side
    pub const fn bottom_side_stop(pixel: Pixel) -> Self {
        Self::new(pixel.bottom_neighbor(), CornerName::TopStart)
    }

    // Stop corner of bottom right side
    pub const fn bottom_right_side_stop(pixel: Pixel) -> Self {
        Self::new(pixel.bottom_right_neighbor(), CornerName::TopStop)
    }

    /// Stop corner of right side
    pub const fn right_side_stop(pixel: Pixel) -> Self {
        Self::new(pixel, CornerName::TopStart)
    }

    /// See docs/corner_normalization.png
    pub const fn side_stop(side: Side) -> Self {
        match side.name {
            SideName::Top => Self::top_side_stop(side.left_pixel),
            SideName::TopLeft => Self::top_left_side_stop(side.left_pixel),
            SideName::Left => Self::left_side_stop(side.left_pixel),
            SideName::Bottom => Self::bottom_side_stop(side.left_pixel),
            SideName::BottomRight => Self::bottom_right_side_stop(side.left_pixel),
            SideName::Right => Self::right_side_stop(side.left_pixel),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::math::{
        pixel::{Pixel, Side, SideName},
        rect::Rect,
    };

    use super::rect_boundary_sides;

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
                pixel.top_neighbor().bottom_side().start_corner()
            );
            assert_eq!(
                pixel.top_side().stop_corner(),
                pixel.top_left_neighbor().bottom_right_side().stop_corner()
            );

            // top left side
            assert_eq!(
                pixel.top_left_side().stop_corner(),
                pixel.left_neighbor().right_side().stop_corner()
            );
            assert_eq!(
                pixel.top_left_side().stop_corner(),
                pixel.top_left_neighbor().bottom_side().stop_corner()
            );

            // left side
            assert_eq!(
                pixel.left_side().stop_corner(),
                pixel.left_neighbor().bottom_right_side().stop_corner()
            );
            assert_eq!(
                pixel.left_side().stop_corner(),
                pixel.bottom_neighbor().top_side().stop_corner()
            );

            // bottom side
            assert_eq!(
                pixel.bottom_side().stop_corner(),
                pixel.bottom_right_neighbor().top_left_side().stop_corner()
            );
            assert_eq!(
                pixel.bottom_side().stop_corner(),
                pixel.bottom_neighbor().right_side().stop_corner()
            );

            // bottom right side
            assert_eq!(
                pixel.bottom_right_side().stop_corner(),
                pixel.right_neighbor().left_side().stop_corner()
            );
            assert_eq!(
                pixel.bottom_right_side().stop_corner(),
                pixel.bottom_right_neighbor().top_side().stop_corner()
            );

            // right side
            assert_eq!(
                pixel.right_side().stop_corner(),
                pixel.right_neighbor().top_left_side().stop_corner()
            );
            assert_eq!(
                pixel.right_side().stop_corner(),
                pixel.top_neighbor().bottom_side().stop_corner()
            );
        }
    }

    /// The smallest side of a pixel p must be smaller than all sides of pixels q where q >= p.
    #[test]
    fn side_pixel_order() {
        let p = Pixel::new(0, 0);
        let &min_side = p.sides_ccw().iter().min().unwrap();
        let neighbors = [
            p.right_neighbor(),
            p.bottom_neighbor(),
            p.bottom_right_neighbor(),
        ];
        for neighbor in neighbors {
            for side in neighbor.sides_ccw() {
                assert!(min_side < side);
            }
        }
    }

    /// The start corner of the smallest ccw side should be equal to the start corner of the
    /// smallest cw side.
    #[test]
    fn min_side_cw_ccw() {
        let p = Pixel::new(0, 0);
        let &ccw_min_side = p.sides_ccw().iter().min().unwrap();
        let &cw_min_side = p.sides_cw().iter().min().unwrap();
        assert_eq!(ccw_min_side.start_corner(), cw_min_side.start_corner());
    }

    /// Test rect_boundary_sides
    #[test]
    fn test_rect_boundary_sides() {
        let rects: [Rect<i64>; 3] = [
            Rect::low_size([0, 0], [1, 1]),
            Rect::low_size([-1, 1], [2, 1]),
            Rect::low_size([-2, 2], [5, 7]),
        ];

        for rect in rects {
            for side in rect_boundary_sides(rect) {
                assert!(rect.half_open_contains(side.left_pixel));
                assert!(!rect.half_open_contains(side.right_pixel()));
            }
        }
    }
}
