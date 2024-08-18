use std::collections::BTreeSet;

use crate::math::point::Point;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaCover {
    // TODO: Use Vec<Point<i64>>
    tiles: BTreeSet<Point<i64>>,
}

impl AreaCover {
    pub fn new() -> Self {
        Self {
            tiles: BTreeSet::new(),
        }
    }

    pub fn iter_tiles<'a>(&'a self) -> impl Iterator<Item = Point<i64>> + Clone + 'a {
        self.tiles.iter().copied()
    }

    pub fn add_tile(&mut self, tile_index: Point<i64>) -> bool {
        let did_insert = self.tiles.insert(tile_index);
        did_insert
    }
}
