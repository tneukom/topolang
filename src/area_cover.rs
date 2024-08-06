use std::collections::BTreeSet;

use crate::{math::point::Point, utils::IteratorPlus};

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

    pub fn iter_tiles<'a>(&'a self) -> impl IteratorPlus<Point<i64>> + 'a {
        self.tiles.iter().copied()
    }

    pub fn add_tile(&mut self, tile_index: Point<i64>) -> bool {
        let did_insert = self.tiles.insert(tile_index);
        did_insert
    }
}
