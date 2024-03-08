use std::collections::BTreeMap;

use crate::{
    bitmap::Bitmap,
    frozen::{counter, Frozen},
    math::{generic::EuclidDivRem, point::Point},
    topology::{Interior, Topology},
    utils::IteratorPlus,
};

/// Alternative names: TileCache, TileWarehouse
pub struct TileStore {
    pub tiles: BTreeMap<Point<i64>, Frozen<Bitmap>>,
    pub updated_time: usize,
}

impl TileStore {
    const TILE_SIZE: usize = 256;

    pub fn new() -> Self {
        Self {
            tiles: BTreeMap::new(),
            updated_time: 0,
        }
    }

    /// Returns all tiles with `modified_time >= after`
    pub fn iter_modified_after<'a>(
        &'a self,
        after: usize,
    ) -> impl IteratorPlus<(&Point<i64>, &Frozen<Bitmap>)> + 'a {
        self.tiles
            .iter()
            .filter(move |(_, frozen)| Frozen::modified_time(frozen) >= after)
    }

    fn fill_interior(&mut self, interior: &Interior) {
        // TODO: Should not freeze every time a pixel is set. But for now it's ok, freeze is only
        //   incrementing a global counter. Might be easier to have already tiled Interior

        for pixel in &interior.pixels {
            let tile_index = pixel.point().euclid_div(Self::TILE_SIZE as i64);
            let frozen_tile = self.tiles.entry(tile_index).or_insert_with(|| {
                let bitmap = Bitmap::transparent(Self::TILE_SIZE, Self::TILE_SIZE);
                Frozen::new(bitmap)
            });

            Frozen::modify(frozen_tile, |tile| {
                let pixel_in_tile: Point<usize> = pixel
                    .point()
                    .euclid_rem(Self::TILE_SIZE as i64)
                    .cwise_try_into()
                    .unwrap();
                tile[pixel_in_tile] = interior.color;
            });
        }
    }

    pub fn update(&mut self, topology: &Topology) {
        // Redraw all region interiors that have changed since the previous update
        for region in topology.iter_region_values() {
            if Frozen::modified_time(&region.interior) > self.updated_time {
                // println!("updating interior {}", Frozen::modified_time(&region.interior));
                self.fill_interior(&region.interior)
            }
        }

        self.updated_time = counter();
    }
}
