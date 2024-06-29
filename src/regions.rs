use crate::{
    field::{Field, FieldIndex},
    math::{
        generic::EuclidDivRem,
        interval::Interval,
        pixel::Side,
        point::Point,
        rect::{Rect, RectBounds},
    },
    utils::IteratorPlus,
};
use std::{collections::BTreeMap, rc::Rc};

/// PartialEq for Rc<T: Eq> checks pointer equality first, see
/// https://github.com/rust-lang/rust/blob/ec1b69852f0c24ae833a74303800db2229b6653e/library/alloc/src/rc.rs#L2254
/// Therefore tile_a == tile_b will return true immediately if Rc::ptr_eq(tile_a, tile_b). This
/// is an important optimization for Pixmap comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tile2<T> {
    field: Field<Option<T>>,
}

impl<T> Tile2<T> {
    pub fn iter<'a>(&'a self) -> impl IteratorPlus<(Point<i64>, &T)> + 'a {
        self.field
            .enumerate()
            .filter_map(|(index, opt_value)| opt_value.as_ref().map(|value| (index, value)))
    }

    /// Panics if index is out of bounds
    pub fn set(&mut self, index: impl FieldIndex, value: T) -> Option<T> {
        self.field.set(index, Some(value))
    }

    pub fn remove(&mut self, index: impl FieldIndex) -> Option<T> {
        self.field.set(index, None)
    }

    pub fn get(&self, index: impl FieldIndex) -> Option<&T> {
        self.field.get(index).unwrap().as_ref()
    }

    pub fn into_map<S>(self, mut f: impl FnMut(T) -> S) -> Tile2<S> {
        let field = self.field.into_map(|opt| opt.map(&mut f));
        Tile2 { field }
    }
}

impl<T: Clone> Tile2<T> {
    pub fn new() -> Self {
        let field = Field::filled(TILE_BOUNDS, None);
        Self { field }
    }

    pub fn filled(value: T) -> Self {
        let field = Field::filled(TILE_BOUNDS, Some(value));
        Self { field }
    }

    pub fn map<S>(&self, mut f: impl FnMut(&T) -> S) -> Tile2<S> {
        let field = self.field.map(|value| value.as_ref().map(&mut f));
        Tile2 { field }
    }

    /// Does not drop tiles that become all None
    pub fn filter_map<S>(&self, mut f: impl FnMut(&T) -> Option<S>) -> Tile2<S> {
        let field = self.field.map(|value| value.as_ref().and_then(&mut f));
        Tile2 { field }
    }

    pub fn keys<'a>(&'a self) -> impl IteratorPlus<Point<i64>> + 'a {
        self.field
            .enumerate()
            .filter_map(|(index, opt_value)| opt_value.as_ref().map(|_| index))
    }

    pub fn blit_if(&mut self, other: &Self, mut pred: impl FnMut(&Option<T>) -> bool) {
        for (self_value, other_value) in self.field.iter_mut().zip(other.field.iter()) {
            if other_value.is_some() && pred(self_value) {
                *self_value = other_value.clone();
            }
        }
    }

    pub fn blit_op<S>(&mut self, other: &Tile2<S>, mut op: impl FnMut(&mut Option<T>, &S)) {
        for (self_value, other_value) in self.field.iter_mut().zip(other.field.iter()) {
            if let Some(other_value) = other_value {
                op(self_value, other_value)
            }
        }
    }

    pub fn fill_none(&self, fill: T) -> Field<T> {
        self.field
            .map(|opt_value| opt_value.clone().unwrap_or_else(|| fill.clone()))
    }
}

const TILE_SIZE: i64 = 64;
const TILE_BOUNDS: Rect<i64> = Rect::new(Interval::new(0, TILE_SIZE), Interval::new(0, TILE_SIZE));

/// Pixel rectangle of the given tile index
pub fn tile_rect(tile_index: Point<i64>) -> Rect<i64> {
    TILE_BOUNDS + tile_index * TILE_SIZE
}

pub fn iter_sides_in_rect(rect: Rect<i64>) -> impl IteratorPlus<Side> {
    let pixel_iter = rect.iter_half_open();
    pixel_iter.flat_map(|pixel| pixel.sides_ccw().into_iter())
}

/// Returned rect contains indices of tiles that cover rect.
pub fn tile_cover(rect: Rect<i64>) -> Rect<i64> {
    let low = tile_index(rect.low());
    // tile_index of the highest index, +1 because upper bound is exclusive.
    let high = tile_index(rect.high() - 1) + 1;
    Rect::low_high(low, high)
}

/// Split pixel index into tile and pixel index
pub fn split_index(index: impl FieldIndex) -> (Point<i64>, Point<i64>) {
    let index = index.point();
    let tile_index = index.euclid_div(TILE_SIZE);
    let pixel_index = index.euclid_rem(TILE_SIZE);
    (tile_index, pixel_index)
}

/// Split pixel index into tile and pixel index
pub fn tile_index(index: Point<i64>) -> Point<i64> {
    index.euclid_div(TILE_SIZE)
}

pub fn combine_indices(tile_index: Point<i64>, offset_index: Point<i64>) -> Point<i64> {
    tile_index * TILE_SIZE + offset_index
}

#[derive(Debug, Clone)]
struct Pixmap2<T> {
    tiles: BTreeMap<Point<i64>, Rc<Tile2<T>>>,
}

/// 3 by 3 grid of Tile references for fast lookups
#[derive(Debug, Clone)]
struct Neighborhood<'a, T> {
    /// Neighborhood of this tile
    tile_index: Point<i64>,

    /// Neighbor tiles of tile_index
    tiles: Field<Option<&'a Tile2<T>>>,
}

#[derive(Debug, Clone)]
struct SideNeighbors<'a, T> {
    side: Side,
    left: Option<&'a T>,
    right: Option<&'a T>,
}

impl<'a, T: Clone> Neighborhood<'a, T> {
    pub fn new(pixmap: &'a Pixmap2<T>, center: Point<i64>) -> Self {
        let bounds = Rect::low_high(center - Point(1, 1), center + Point(1, 1));
        let tiles = Field::from_map(bounds, |tile_index| pixmap.get_tile(tile_index));
        Self {
            tile_index: center,
            tiles,
        }
    }

    pub fn iter_sides(&self) -> impl IteratorPlus<Side> {
        let tile_rect = tile_rect(self.tile_index);
        iter_sides_in_rect(tile_rect)
    }

    /// Panics if index is out of bounds
    pub fn get(&self, index: impl FieldIndex) -> Option<&'a T> {
        let (tile_index, pixel_index) = split_index(index);
        let tile = self.tiles[tile_index]?;
        tile.get(pixel_index)
    }

    pub fn into_iter_side_neighbors(self) -> impl IteratorPlus<SideNeighbors<'a, T>> {
        self.iter_sides().map(move |side| {
            let left = self.get(side.left_pixel);
            let right = self.get(side.right_pixel());
            SideNeighbors { side, left, right }
        })
    }
}

impl<T: Clone> Pixmap2<T> {
    pub fn new() -> Self {
        let tiles = BTreeMap::new();
        Self { tiles }
    }

    pub fn iter(&self) -> impl IteratorPlus<(Point<i64>, &T)> {
        self.tiles.iter().flat_map(|(&tile_index, tile)| {
            tile.iter().map(move |(offset_index, value)| {
                (combine_indices(tile_index, offset_index), value)
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
        RectBounds::iter_bounds(self.keys()).inc_high()
    }

    pub fn get_tile(&self, tile_index: Point<i64>) -> Option<&Tile2<T>> {
        Some(self.tiles.get(&tile_index)?.as_ref())
    }

    // If no tile exists at the given location a new one is created.
    fn get_tile_mut(&mut self, tile_index: Point<i64>) -> &mut Tile2<T> {
        let rc_tile = self
            .tiles
            .entry(tile_index)
            .or_insert_with(|| Rc::new(Tile2::new()));
        Rc::make_mut(rc_tile)
    }

    /// Expensive, try to do bulk operations Tile by Tile
    pub fn get(&self, index: impl FieldIndex) -> Option<&T> {
        let (tile_index, pixel_index) = split_index(index.point());
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
        for tile_index in tile_cover(field.bounds()).iter_half_open() {
            // The pixel rect of the given tile
            let tile_rect = tile_rect(tile_index);
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
            let tile_rect = tile_rect(tile_index);
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

    /// Iterate tile with their neighborhoods
    pub fn iter_neighborhoods(&self) -> impl IteratorPlus<Neighborhood<T>> {
        self.tiles
            .keys()
            .map(|&tile_index| Neighborhood::new(self, tile_index))
    }

    /// Iterate over all sides of set pixels with their left and right neighbors.
    pub fn iter_side_neighbors<'a>(&'a self) -> impl Iterator<Item = SideNeighbors<'a, T>> {
        self.iter_neighborhoods()
            .flat_map(|neighborhood| neighborhood.into_iter_side_neighbors())
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
