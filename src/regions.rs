use std::{collections::BTreeMap, rc::Rc};

use crate::{
    field::{Field, FieldIndex},
    math::{generic::EuclidDivRem, point::Point, rect::Rect},
    pixmap::Tile,
    utils::IteratorPlus,
};

struct Pixmap2<T> {
    tiles: BTreeMap<Point<i64>, Rc<Tile<T>>>,
}

impl<T: Clone> Pixmap2<T> {
    /// Split pixel index into tile and pixel index
    pub fn tile_index(index: Point<i64>) -> Point<i64> {
        index.euclid_div(Tile::<T>::SIZE)
    }

    /// Pixel rectangle of the given tile index
    pub fn tile_rect(tile_index: Point<i64>) -> Rect<i64> {
        Tile::<T>::BOUNDS + tile_index * Tile::<T>::SIZE
    }

    /// Returned rect contains indices of tiles that cover rect.
    pub fn tile_cover(rect: Rect<i64>) -> Rect<i64> {
        let low = Self::tile_index(rect.low());
        // tile_index of the highest index, +1 because upper bound is exclusive.
        let high = Self::tile_index(rect.high() - 1) + 1;
        Rect::low_high(low, high)
    }

    /// Split pixel index into tile and pixel index
    pub fn split_index(index: Point<i64>) -> (Point<i64>, Point<i64>) {
        let tile_index = index.euclid_div(Tile::<T>::SIZE);
        let pixel_index = index.euclid_rem(Tile::<T>::SIZE);
        (tile_index, pixel_index)
    }

    pub fn combine_indices(tile_index: Point<i64>, offset_index: Point<i64>) -> Point<i64> {
        tile_index * Tile::<T>::SIZE + offset_index
    }

    pub fn new() -> Self {
        let tiles = BTreeMap::new();
        Self { tiles }
    }

    pub fn iter(&self) -> impl IteratorPlus<(Point<i64>, &T)> {
        self.tiles.iter().flat_map(|(&tile_index, tile)| {
            tile.iter().map(move |(offset_index, value)| {
                (Self::combine_indices(tile_index, offset_index), value)
            })
        })
    }

    pub fn keys(&self) -> impl IteratorPlus<Point<i64>> + '_ {
        self.iter().map(|(index, _)| index)
    }

    pub fn values(&self) -> impl IteratorPlus<&T> {
        self.iter().map(|(_, value)| value)
    }

    pub fn bounding_rect(&self) -> Rect<i64> {
        // TODO: In many cases returning the bounding rectangle of the tiles would be enough
        // TODO:SPEEDUP: There must be a quicker way than just brute force
        crate::math::rect::RectBounds::iter_bounds(self.keys()).inc_high()
    }

    // If no tile exists at the given location a new one is created.
    fn get_tile_mut(&mut self, tile_index: Point<i64>) -> &mut Tile<T> {
        let rc_tile = self
            .tiles
            .entry(tile_index)
            .or_insert_with(|| Rc::new(Tile::new()));
        Rc::make_mut(rc_tile)
    }

    /// Expensive, try to do bulk operations Tile by Tile
    pub fn get(&self, index: impl FieldIndex) -> Option<&T> {
        let (tile_index, pixel_index) = Self::split_index(index.point());
        let tile = self.tiles.get(&tile_index)?.as_ref();
        tile.get(pixel_index)
    }

    pub fn map<S>(&self, mut f: impl FnMut(&T) -> S) -> Pixmap2<S> {
        let tiles: BTreeMap<_, _> = self
            .tiles
            .iter()
            .map(|(&index, tile)| {
                let mapped_tile = tile.as_ref().map(&mut f);
                (index, Rc::new(mapped_tile))
            })
            .collect();
        Pixmap2 { tiles }
    }

    pub fn blit_field(&mut self, field: &Field<T>) {
        for tile_index in Self::tile_cover(field.bounds()).iter_half_open() {
            // The pixel rect of the given tile
            let tile_rect = Self::tile_rect(tile_index);
            let tile = self.get_tile_mut(tile_index);
            for pixel_index in tile_rect.iter_half_open() {
                if let Some(value) = field.get(pixel_index) {
                    tile.set(pixel_index - tile_rect.low(), value.clone());
                }
            }
        }
    }

    pub fn from_field(field: &Field<T>) -> Self {
        let mut pixmap = Self::new();
        pixmap.blit_field(field);
        pixmap
    }

    pub fn blit_to_field(&self, field: &mut Field<T>) {
        for (&tile_index, tile) in &self.tiles {
            let tile_rect = Self::tile_rect(tile_index);
            for pixel_index in tile_rect.iter_half_open() {
                if let Some(linear_index) = field.linear_index(pixel_index) {
                    field.as_mut_slice()[linear_index] =
                        tile.get(pixel_index - tile_rect.low()).unwrap().clone();
                }
            }
        }
    }

    pub fn to_field(&self, default: T) -> Field<T> {
        let bounds = self.bounding_rect();
        let mut field = Field::filled(bounds, default);
        self.blit_to_field(&mut field);
        field
    }
}

// // TODO: Generic over Id type?
// pub fn field_regions<T: Eq + Copy>(colors: &Field<T>) -> Field<usize> {
//     let mut union_find: UnionFind<usize> = UnionFind::new(colors.len());
//     for (index, &color) in colors.enumerate() {
//         for side_name in [SideName::Right, SideName::Bottom, SideName::BottomLeft] {
//             let neighbor_index = index.neighbor(side_name);
//             let neighbor_color = colors.get(neighbor_index);
//             if Some(color) == neighbor_color {
//                 // same region, merge
//                 union_find.union(
//                     colors.linear_index(index).unwrap(),
//                     colors.linear_index(neighbor_index).unwrap(),
//                 );
//             }
//         }
//     }
// }
//
// struct FieldRegions {}
//
// pub fn pixmap_regions<T: Eq + Copy>(colors: &Pixmap<T>) {
//     // TODO:SPEEDUP: Use faster HashMap
//     let tile_regions: HashMap<Point<i64>, FieldRegions> = todo!("Calculate regions for all tiles");
//
//     // Count total number of regions
//     // let tile_region_count = tile_regions.values().map(|tile_region| tile_region.len());
//
//     todo!("Create UnionFind with total number of regions");
//     todo!("Merge UnionFind regions if they have the same colors and share a side");
//     todo!("Merge sides Sets and create Regions for the whole Pixmap");
// }

#[cfg(test)]
mod test {
    use crate::{
        field::Field,
        math::{generic::EuclidDivRem, rect::Rect, rgba8::Rgba8},
        pixmap::Tile,
        regions::Pixmap2,
    };

    fn generate_field(bounds: Rect<i64>) -> Field<Rgba8> {
        Field::from_map(bounds, |index| {
            let red = index.x.euclid_rem(256) as u8;
            let green = index.y.euclid_rem(256) as u8;
            Rgba8::new(red, green, 255, 255)
        })
    }

    fn field_bounds_cases() -> Vec<Rect<i64>> {
        let tile_size = Tile::<u8>::SIZE;
        return vec![
            Rect::low_high([0, 0], [1, 1]),
            Rect::low_high([-1, 1], [0, 0]),
            Rect::low_high([0, 0], [tile_size - 1, tile_size - 1]),
            Rect::low_high([0, 0], [tile_size + 1, tile_size - 1]),
            Rect::low_high([-391, 234], [519, 748]),
        ];
    }

    #[test]
    fn convert_field() {
        for bounds in field_bounds_cases() {
            let bounds = Rect::low_size([0, 0], [10, 10]);
            let field = generate_field(bounds);
            let pixmap = Pixmap2::from_field(&field);
            assert_eq!(pixmap.bounding_rect(), bounds);

            for (index, value) in field.enumerate() {
                assert_eq!(Some(value), pixmap.get(index));
            }

            assert_eq!(pixmap.to_field(Rgba8::ZERO), field);
        }
    }
}
