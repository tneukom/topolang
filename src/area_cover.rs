use std::collections::BTreeSet;

use crate::{
    math::{point::Point, rect::Rect},
    pixmap,
    utils::IteratorPlus,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaCover {
    // TODO: Use Vec<Point<i64>>
    tiles: BTreeSet<Point<i64>>,

    /// Important: The upper bound is inclusive to make incremental update easier
    bounding_rect: Rect<i64>,
}

impl AreaCover {
    pub fn new() -> Self {
        Self {
            tiles: BTreeSet::new(),
            bounding_rect: Rect::EMPTY,
        }
    }

    pub fn iter_tiles<'a>(&'a self) -> impl IteratorPlus<Point<i64>> + 'a {
        self.tiles.iter().copied()
    }

    pub fn add(&mut self, index: Point<i64>) -> bool {
        let (tile_index, _) = pixmap::split_index(index);
        let did_insert = self.tiles.insert(tile_index);

        // TODO:SPEEDUP: bounds_with(...) does more work than necessary here.
        self.bounding_rect = self.bounding_rect.bounds_with(index);

        did_insert
    }

    /// Bounding box of included pixels, upper bound is exclusive.
    pub fn bounding_rect(&self) -> Rect<i64> {
        self.bounding_rect.inc_high()
    }
}
