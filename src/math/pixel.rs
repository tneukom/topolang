use crate::math::{
    direction::{DiagonalDirection, Direction},
    point::Point,
};
use std::{
    fmt::{Debug, Display, Formatter},
    marker::PhantomData,
    ops::{Add, Sub},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GridPoint<T> {
    pub x: i64,
    pub y: i64,
    _grid_type: PhantomData<T>,
}

impl<T> GridPoint<T> {
    pub fn new(x: i64, y: i64) -> Self {
        Self {
            x,
            y,
            _grid_type: PhantomData::default(),
        }
    }

    pub fn point(self) -> Point<i64> {
        Point::new(self.x, self.y)
    }

    pub fn left(self) -> Self {
        Self::new(self.x - 1, self.y)
    }

    pub fn right(self) -> Self {
        Self::new(self.x + 1, self.y)
    }

    pub fn up(self) -> Self {
        Self::new(self.x, self.y - 1)
    }

    pub fn down(self) -> Self {
        Self::new(self.x, self.y + 1)
    }

    pub fn neighbor(self, direction: Direction) -> Self {
        match direction {
            Direction::Left => self.left(),
            Direction::Right => self.right(),
            Direction::Up => self.up(),
            Direction::Down => self.down(),
        }
    }

    pub fn up_left(self) -> Self {
        Self::new(self.x - 1, self.y - 1)
    }

    pub fn up_right(self) -> Self {
        Self::new(self.x + 1, self.y - 1)
    }

    pub fn down_left(self) -> Self {
        Self::new(self.x - 1, self.y + 1)
    }

    pub fn down_right(self) -> Self {
        Self::new(self.x + 1, self.y + 1)
    }

    pub fn diagonal_neighbor(self, direction: DiagonalDirection) -> Self {
        match direction {
            DiagonalDirection::UpLeft => self.up_left(),
            DiagonalDirection::UpRight => self.up_right(),
            DiagonalDirection::DownLeft => self.down_left(),
            DiagonalDirection::DownRight => self.down_right(),
        }
    }
}

impl<T> From<Point<i64>> for GridPoint<T> {
    fn from(value: Point<i64>) -> Self {
        GridPoint::new(value.x, value.y)
    }
}

impl<T> Add<Point<i64>> for GridPoint<T> {
    type Output = GridPoint<T>;

    fn add(self, rhs: Point<i64>) -> Self::Output {
        GridPoint::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<T> Sub<GridPoint<T>> for GridPoint<T> {
    type Output = Point<i64>;

    fn sub(self, rhs: GridPoint<T>) -> Self::Output {
        Point::new(self.x - rhs.x, self.y - rhs.y)
    }
}

#[derive(Clone, Copy, Debug, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct PixelGrid {}

#[derive(Clone, Copy, Debug, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct CornerGrid {}

pub type Pixel = GridPoint<PixelGrid>;

pub type Corner = GridPoint<CornerGrid>;

impl Pixel {
    pub fn top_left_corner(self) -> Corner {
        Corner::new(self.x, self.y)
    }

    pub fn top_right_corner(self) -> Corner {
        Corner::new(self.x + 1, self.y)
    }

    pub fn bottom_left_corner(self) -> Corner {
        Corner::new(self.x, self.y + 1)
    }

    pub fn bottom_right_corner(self) -> Corner {
        Corner::new(self.x + 1, self.y + 1)
    }

    pub fn corner(self, direction: DiagonalDirection) -> Corner {
        match direction {
            DiagonalDirection::UpLeft => self.top_left_corner(),
            DiagonalDirection::UpRight => self.top_right_corner(),
            DiagonalDirection::DownLeft => self.bottom_left_corner(),
            DiagonalDirection::DownRight => self.bottom_right_corner(),
        }
    }

    pub fn left_side_ccw(self) -> Side {
        Side::new(self.top_left_corner(), Direction::Down)
    }

    pub fn bottom_side_ccw(self) -> Side {
        Side::new(self.bottom_left_corner(), Direction::Right)
    }

    pub fn right_side_ccw(self) -> Side {
        Side::new(self.bottom_right_corner(), Direction::Up)
    }

    pub fn top_side_ccw(self) -> Side {
        Side::new(self.top_right_corner(), Direction::Left)
    }

    pub fn side_ccw(self, direction: Direction) -> Side {
        match direction {
            Direction::Left => self.left_side_ccw(),
            Direction::Right => self.right_side_ccw(),
            Direction::Up => self.top_side_ccw(),
            Direction::Down => self.bottom_side_ccw(),
        }
    }

    pub fn sides_ccw(self) -> [Side; 4] {
        [
            self.left_side_ccw(),
            self.bottom_side_ccw(),
            self.right_side_ccw(),
            self.top_side_ccw(),
        ]
    }
}

impl Corner {
    pub fn top_left_pixel(self) -> Pixel {
        Pixel::new(self.x - 1, self.y - 1)
    }

    pub fn top_right_pixel(self) -> Pixel {
        Pixel::new(self.x, self.y - 1)
    }

    pub fn bottom_left_pixel(self) -> Pixel {
        Pixel::new(self.x - 1, self.y)
    }

    pub fn bottom_right_pixel(self) -> Pixel {
        Pixel::new(self.x, self.y)
    }

    pub fn neighbor_pixel(self, direction: DiagonalDirection) -> Pixel {
        match direction {
            DiagonalDirection::UpLeft => self.top_left_pixel(),
            DiagonalDirection::UpRight => self.top_right_pixel(),
            DiagonalDirection::DownLeft => self.bottom_left_pixel(),
            DiagonalDirection::DownRight => self.bottom_right_pixel(),
        }
    }
}

impl Display for GridPoint<PixelGrid> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pixel({}, {})", self.x, self.y)
    }
}

impl Display for GridPoint<CornerGrid> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Corner({}, {})", self.x, self.y)
    }
}

/// Directed side, has start and end corners
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct Side {
    corner: Corner,
    direction: Direction,
}

impl Side {
    pub fn new(corner: Corner, direction: Direction) -> Self {
        Side { corner, direction }
    }

    pub fn start_corner(self) -> Corner {
        self.corner
    }

    pub fn stop_corner(self) -> Corner {
        self.corner.neighbor(self.direction)
    }

    pub fn reversed(self) -> Self {
        Self::new(
            self.corner.neighbor(self.direction),
            self.direction.reversed(),
        )
    }

    pub fn left_pixel(self) -> Pixel {
        match self.direction {
            Direction::Left => self.corner.bottom_left_pixel(),
            Direction::Right => self.corner.top_right_pixel(),
            Direction::Up => self.corner.top_left_pixel(),
            Direction::Down => self.corner.bottom_right_pixel(),
        }
    }

    pub fn right_pixel(self) -> Pixel {
        self.reversed().left_pixel()
    }
}

impl Display for Side {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Side({}, {}, {})",
            self.corner.x,
            self.corner.y,
            self.direction.unicode_symbol()
        )
    }
}
