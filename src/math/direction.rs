use crate::{
    math::{
        arrow::Arrow,
        axis_line::AxisArrow,
        generic::Num,
        point::{pt, Point},
    },
    utils::ReflectEnum,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    pub const ALL: [Self; 4] = [Self::Left, Self::Right, Self::Up, Self::Down];

    pub fn reverse(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }

    /// A step of length one in the given direction
    pub fn unit_step(self) -> Point<i64> {
        match self {
            Self::Left => pt(-1, 0),
            Self::Right => pt(1, 0),
            Self::Up => pt(0, -1),
            Self::Down => pt(0, 1),
        }
    }

    /// returns self such that delta = k * self.unit_step() for some k != 0 or None otherwise
    pub fn from_dir<T: Num>(delta: Point<T>) -> Option<Self> {
        match delta {
            delta if delta.x < T::ZERO && delta.y == T::ZERO => Some(Self::Left),
            delta if delta.x > T::ZERO && delta.y == T::ZERO => Some(Self::Right),
            delta if delta.x == T::ZERO && delta.y < T::ZERO => Some(Self::Up),
            delta if delta.x == T::ZERO && delta.y > T::ZERO => Some(Self::Down),
            _ => None,
        }
    }

    pub fn from_arrow<T: Num>(arrow: Arrow<T>) -> Option<Self> {
        Self::from_dir(arrow.dir())
    }

    pub fn from_axis_arrow<T: Num>(arrow: AxisArrow<T>) -> Option<Self> {
        match arrow {
            AxisArrow::Vertical { y, .. } => {
                if y.is_increasing() {
                    Some(Self::Down)
                } else if y.is_decreasing() {
                    Some(Self::Up)
                } else {
                    None
                }
            }
            AxisArrow::Horizontal { x, .. } => {
                if x.is_increasing() {
                    Some(Self::Right)
                } else if x.is_decreasing() {
                    Some(Self::Left)
                } else {
                    None
                }
            }
        }
    }
}

impl ReflectEnum for Direction {
    fn all() -> &'static [Self] {
        &Self::ALL
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Up => "Up",
            Self::Down => "Down",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DiagonalDirection {
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

impl DiagonalDirection {
    pub const ALL: [Self; 4] = [Self::UpLeft, Self::UpRight, Self::DownLeft, Self::DownRight];
}
