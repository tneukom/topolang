use crate::math::pixel::Corner;
use crate::topology::{BorderKey, RegionKey, Seam};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Element {
    Seam(Seam),
    Corner(Corner),
    Region(RegionKey),
    Border(BorderKey),
}

impl From<Seam> for Element {
    fn from(seam: Seam) -> Self {
        Self::Seam(seam)
    }
}

impl From<Corner> for Element {
    fn from(corner: Corner) -> Self {
        Self::Corner(corner)
    }
}

impl From<RegionKey> for Element {
    fn from(region_key: RegionKey) -> Self {
        Self::Region(region_key)
    }
}

impl From<BorderKey> for Element {
    fn from(border_key: BorderKey) -> Self {
        Self::Border(border_key)
    }
}