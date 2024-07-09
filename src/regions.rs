/// Unionfind crates
/// https://crates.io/crates/union-find
/// https://crates.io/crates/disjoint-sets
///
/// https://crates.io/crates/partitions
/// Has clear function
///
/// https://docs.rs/petgraph/latest/src/petgraph/unionfind.rs.html#16-27
use crate::{
    field::{Field, FieldIndex},
    math::{
        interval::Interval,
        pixel::{Side, SideName},
        point::Point,
        rect::{Rect, RectBounds},
        rgba8::Rgba8,
    },
    utils::IteratorPlus,
};
use petgraph::unionfind::UnionFind;
use std::{cell::Cell, collections::BTreeMap, ops::Deref, rc::Rc};

/// PartialEq for Rc<T: Eq> checks pointer equality first, see
/// https://github.com/rust-lang/rust/blob/ec1b69852f0c24ae833a74303800db2229b6653e/library/alloc/src/rc.rs#L2254
/// Therefore tile_a == tile_b will return true immediately if Rc::ptr_eq(tile_a, tile_b). This
/// is an important optimization for Pixmap comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tile2<T> {
    field: Field<Option<T>>,
}

impl<T> Tile2<T> {
    pub fn new() -> Self {
        let field = Field::from_map(TILE_BOUNDS, |_| None);
        Self { field }
    }

    pub fn iter<'a>(&'a self) -> impl IteratorPlus<(Point<i64>, &T)> + 'a {
        self.field
            .enumerate()
            .filter_map(|(index, opt_value)| opt_value.as_ref().map(|value| (index, value)))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Point<i64>, &mut T)> {
        self.field
            .enumerate_mut()
            .filter_map(|(index, opt_value)| opt_value.as_mut().map(|value| (index, value)))
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

    /// All items are None
    pub fn is_empty(&self) -> bool {
        self.field.iter().all(|item| item.is_none())
    }
}

impl<T: Clone> Tile2<T> {
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
    // Using bit manipulation for TILE_SIZE == 64
    const {
        assert!(TILE_SIZE == 64);
    }

    let index = index.point();

    // TODO: Make sure this optimization works
    // let tile_index = index.euclid_div(TILE_SIZE);
    let tile_index = Point(index.x >> 6, index.y >> 6);
    // let pixel_index = index.euclid_rem(TILE_SIZE);
    let pixel_index = Point(index.x & 0b111111, index.y & 0b111111);

    (tile_index, pixel_index)
}

/// Split pixel index into tile and pixel index
pub fn tile_index(index: Point<i64>) -> Point<i64> {
    // TODO: Make sure this optimization works
    // index.euclid_div(TILE_SIZE)
    Point(index.x >> 6, index.y >> 6)
}

pub fn combine_indices(tile_index: Point<i64>, offset_index: Point<i64>) -> Point<i64> {
    tile_index * TILE_SIZE + offset_index
}

/// 3 by 3 grid of Tile references for fast lookups
#[derive(Debug, Clone)]
pub struct Neighborhood<'a, T> {
    /// Neighborhood of this tile
    tile_index: Point<i64>,

    /// Neighbor tiles of tile_index
    tiles: Field<Option<&'a Tile2<T>>>,
}

#[derive(Debug, Clone)]
pub struct SideNeighbors<'a, T> {
    pub side: Side,
    pub left: Option<&'a T>,
    pub right: Option<&'a T>,
}

pub struct LowNeighbors<'a, T> {
    top: Option<&'a T>,
    left: Option<&'a T>,
    top_right: Option<&'a T>,
}

impl<'a, T> LowNeighbors<'a, Tile2<T>> {
    pub fn tile_neighbors(pixmap2: &'a Pixmap2<T>, tile_index: Point<i64>) -> Self {
        Self {
            top: pixmap2.get_tile(tile_index.top_neighbor()),
            left: pixmap2.get_tile(tile_index.left_neighbor()),
            top_right: pixmap2.get_tile(tile_index.top_left_neighbor()),
        }
    }
}

struct LowNeighborhood<'a, T> {
    tiles: [[Option<&'a Tile2<T>>; 3]; 2],
}

impl<'a, T> LowNeighborhood<'a, T> {
    pub fn new(main: &'a Tile2<T>, neighbors: &LowNeighbors<'a, Tile2<T>>) -> Self {
        let tiles = [
            [None, neighbors.top, neighbors.top_right],
            [neighbors.left, Some(main), None],
        ];
        Self { tiles }
    }

    pub fn get(&self, index: Point<i64>) -> Option<&'a T> {
        let (tile_index, pixel_index) = split_index(index);
        assert!(tile_index.x >= -1 && tile_index.x < 2);
        assert!(tile_index.y >= -1 && tile_index.y < 1);
        let tile = self.tiles[(tile_index.y + 1) as usize][(tile_index.x + 1) as usize]?;
        tile.get(pixel_index)
    }

    pub fn get_neighbors(&self, index: Point<i64>) -> LowNeighbors<'a, T> {
        LowNeighbors {
            top: self.get(index.top_neighbor()),
            left: self.get(index.left_neighbor()),
            top_right: self.get(index.top_left_neighbor()),
        }
    }
}

impl<'a, T: Clone> Neighborhood<'a, T> {
    pub fn new(pixmap: &'a Pixmap2<T>, tile_index: Point<i64>) -> Self {
        let bounds = Rect::low_size(tile_index - Point(1, 1), Point(3, 3));
        let tiles = Field::from_map(bounds, |tile_index| pixmap.get_tile(tile_index));
        Self { tile_index, tiles }
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

#[derive(Debug, Clone)]
pub struct Pixmap2<T> {
    tiles: BTreeMap<Point<i64>, Rc<Tile2<T>>>,
}

impl<T> Pixmap2<T> {
    pub fn new() -> Self {
        let tiles = BTreeMap::new();
        Self { tiles }
    }

    pub fn count(&self) -> usize {
        self.iter().count()
    }

    pub fn iter(&self) -> impl IteratorPlus<(Point<i64>, &T)> {
        self.tiles.iter().flat_map(|(&tile_index, tile)| {
            tile.iter().map(move |(offset_index, value)| {
                (combine_indices(tile_index, offset_index), value)
            })
        })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Point<i64>, &mut T)> {
        self.tiles.iter_mut().flat_map(|(&tile_index, tile)| {
            let tile_mut = Rc::get_mut(tile).unwrap();
            tile_mut.iter_mut().map(move |(offset_index, value)| {
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

    /// Expensive, try to do bulk operations Tile by Tile
    pub fn get(&self, index: impl FieldIndex) -> Option<&T> {
        let (tile_index, pixel_index) = split_index(index.point());
        let tile = self.tiles.get(&tile_index)?.as_ref();
        tile.get(pixel_index)
    }
}

impl<T: Clone> Pixmap2<T> {
    // If no tile exists at the given location a new one is created.
    fn get_tile_mut(&mut self, tile_index: Point<i64>) -> &mut Tile2<T> {
        let rc_tile = self
            .tiles
            .entry(tile_index)
            .or_insert_with(|| Rc::new(Tile2::new()));
        Rc::make_mut(rc_tile)
    }

    pub fn map<S>(&self, mut f: impl FnMut(&T) -> S) -> Pixmap2<S> {
        let tiles: BTreeMap<_, _> = self
            .tiles
            .iter()
            .map(|(&index, rc_tile)| {
                let mapped_tile = rc_tile.as_ref().map(&mut f);
                (index, Rc::new(mapped_tile))
            })
            .collect();
        Pixmap2 { tiles }
    }

    pub fn into_map<S>(self, mut f: impl FnMut(T) -> S) -> Pixmap2<S> {
        let tiles: BTreeMap<_, _> = self
            .tiles
            .into_iter()
            .map(|(index, rc_tile)| {
                let mapped_tile = Rc::unwrap_or_clone(rc_tile).into_map(&mut f);
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
    pub fn iter_side_neighbors(&self) -> impl Iterator<Item = SideNeighbors<T>> {
        self.iter_neighborhoods()
            .flat_map(|neighborhood| neighborhood.into_iter_side_neighbors())
    }

    pub fn build_tiles_with_neighbors<S>(
        &self,
        mut f: impl FnMut(&Tile2<T>, LowNeighbors<Tile2<T>>, LowNeighbors<Tile2<S>>) -> Tile2<S>,
    ) -> Pixmap2<S> {
        let mut result_map: Pixmap2<S> = Pixmap2::new();

        for (&tile_index, tile) in &self.tiles {
            let neighbors = LowNeighbors::tile_neighbors(self, tile_index);
            let result_neighbors = LowNeighbors::tile_neighbors(&result_map, tile_index);

            let result_tile = f(tile.deref(), neighbors, result_neighbors);
            result_map.tiles.insert(tile_index, Rc::new(result_tile));
        }

        result_map
    }

    pub fn build_with_neighbors<S>(
        &self,
        mut f: impl FnMut(&T, LowNeighbors<T>, LowNeighbors<S>) -> S,
    ) -> Pixmap2<S> {
        self.build_tiles_with_neighbors(|tile, tile_neighbors, result_tile_neighbors| {
            let mut result_tile = Tile2::new();

            for (pixel_index, value) in tile.iter() {
                let result = {
                    let result_neighbors =
                        LowNeighborhood::new(&result_tile, &result_tile_neighbors)
                            .get_neighbors(pixel_index);

                    let neighbors =
                        LowNeighborhood::new(tile, &tile_neighbors).get_neighbors(pixel_index);

                    f(value, neighbors, result_neighbors)
                };
                result_tile.set(pixel_index, result);
            }

            result_tile
        })
    }

    // pub fn map_with_neighbors<S>(
    //     &self,
    //     mut f: impl FnMut(&T, LowNeighbors<T>, LowNeighbors<S>) -> S,
    // ) -> Pixmap2<S> {
    //     let mut result_map: Pixmap2<S> = Pixmap2::new();
    //
    //     for (&tile_index, tile) in &self.tiles {
    //         let tile_neighbors = LowNeighbors::tile_neighbors(self, tile_index);
    //         let neighborhood = LowNeighborhood::new(tile.as_ref(), &tile_neighbors);
    //         let result_tile_neighbors = LowNeighbors::tile_neighbors(&result_map, tile_index);
    //
    //         let mut result_tile = Tile2::new();
    //
    //         for (pixel_index, value) in tile.iter() {
    //             let result = {
    //                 let result_neighborhood =
    //                     LowNeighborhood::new(&result_tile, &result_tile_neighbors);
    //                 let result_neighbors = result_neighborhood.get_neighbors(pixel_index);
    //
    //                 let neighbors = neighborhood.get_neighbors(pixel_index);
    //
    //                 f(value, neighbors, result_neighbors)
    //             };
    //             result_tile.set(pixel_index, result);
    //         }
    //
    //         result_map.tiles.insert(tile_index, Rc::new(result_tile));
    //     }
    //
    //     result_map
    // }
}

impl<T: PartialEq + Clone> PartialEq for Pixmap2<T> {
    fn eq(&self, other: &Self) -> bool {
        self.tiles.iter().all(|(&tile_index, tile)| {
            let other_tile = other.get_tile(tile_index);
            if let Some(other_tile) = other_tile {
                tile.deref() == other_tile
            } else {
                tile.is_empty()
            }
        })
    }
}

impl From<&Field<Rgba8>> for Pixmap2<Rgba8> {
    fn from(field: &Field<Rgba8>) -> Self {
        let mut pixmap = Self::new();
        pixmap.blit_field(field);
        pixmap
    }
}

impl From<Field<Rgba8>> for Pixmap2<Rgba8> {
    fn from(field: Field<Rgba8>) -> Self {
        Self::from(&field)
    }
}

fn compact_labels(labels: &mut Vec<usize>) {
    let mut remap = vec![usize::MAX; labels.len()];
    let mut label_counter = 0;

    for label in labels {
        if remap[*label] == usize::MAX {
            remap[*label] = label_counter;
            label_counter += 1;
        }
        *label = remap[*label];
    }
}

pub fn field_regions<T: Clone + Eq>(field: &Field<T>) -> Field<usize> {
    let mut union_find = UnionFind::new(field.len());

    for (center, value) in field.enumerate() {
        for side in [SideName::TopLeft, SideName::Top, SideName::Left] {
            let neighbor = center.neighbor(side);
            if Some(value) == field.get(neighbor) {
                union_find.union(
                    field.linear_index(center).unwrap(),
                    field.linear_index(neighbor).unwrap(),
                );
            }
        }
    }

    let mut labeling = union_find.into_labeling();
    compact_labels(&mut labeling);
    Field::from_linear(field.bounds(), labeling)
}

pub fn pixmap_regions2<T: Clone + Eq>(pixmap: &Pixmap2<T>) -> Pixmap2<usize> {
    use petgraph::unionfind::UnionFind;

    let mut union_find = UnionFind::new(pixmap.count());
    let mut counter: usize = 0;
    let mut region_map = pixmap.build_with_neighbors(|value, neighbors, region_neighbors| {
        let id = counter;
        counter += 1;

        if neighbors.top == Some(value) {
            union_find.union(id, *region_neighbors.top.unwrap());
        }
        if neighbors.left == Some(value) {
            union_find.union(id, *region_neighbors.left.unwrap());
        }
        if neighbors.top_right == Some(value) {
            union_find.union(id, *region_neighbors.top_right.unwrap());
        }

        id
    });

    for (_index, id) in region_map.iter_mut() {
        *id = union_find.find(*id);
    }

    region_map
}

pub fn pixmap_regions<T: Clone + Eq>(pixmap: &Pixmap2<T>) -> Pixmap2<(T, usize)> {
    use petgraph::unionfind::UnionFind;

    // assign initial region id
    let mut id: usize = 0;
    let region_map = pixmap.map(|value| {
        let current_id = id;
        id += 1;
        (value.clone(), Cell::new(current_id))
    });

    // merge using UnionFind
    let mut union_find = UnionFind::new(id);

    for side in region_map.iter_side_neighbors() {
        if let Some(left) = side.left {
            if let Some(right) = side.right {
                if left.0 == right.0 {
                    union_find.union(left.1.get(), right.1.get());
                }
            }
        }
    }

    // replace ids
    for value in region_map.values() {
        let union_id = union_find.find(value.1.get());
        value.1.set(union_id);
    }

    // TODO: This probably allocates even though it doesn't need to
    region_map.into_map(|value| (value.0, value.1.get()))
}

#[cfg(test)]
mod test {
    use std::cell::Cell;

    use crate::{
        field::Field,
        math::{generic::EuclidDivRem, pixel::Side, point::Point, rect::Rect, rgba8::Rgba8},
        pixmap::Tile,
        regions::{pixmap_regions2, split_index, Pixmap2, TILE_SIZE},
    };

    pub fn pixel_touches_tile_boundary(index: Point<i64>) -> bool {
        let (_, pixel_index) = split_index(index);
        pixel_index.x == 0
            || pixel_index.x == TILE_SIZE - 1
            || pixel_index.y == 0
            || pixel_index.y == TILE_SIZE - 1
    }

    pub fn side_lies_on_tile_boundary(side: Side) -> bool {
        pixel_touches_tile_boundary(side.left_pixel)
    }

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
            Rect::low_high([-1, -1], [0, 0]),
            Rect::low_high([0, 0], [tile_size - 1, tile_size - 1]),
            Rect::low_high([0, 0], [tile_size + 1, tile_size - 1]),
            Rect::low_high([-391, 234], [519, 748]),
        ];
    }

    fn test_mark_tile_interface(name: &str) {
        let folder = "test_resources/regions/mark_tile_interface";

        let before: Pixmap2<Rgba8> = Field::load(format!("{folder}/{name}_before.png"))
            .unwrap()
            .into();
        // Turn into Pixmap2<Cell<Rgba8>> so we can modify it
        let before_cell = before.into_map(|rgba| Cell::new(rgba));
        before_cell.iter_side_neighbors().for_each(|side| {
            if side.left == Some(&Cell::new(Rgba8::BLUE)) {
                let right = side.right.unwrap();
                if right.get() == Rgba8::TRANSPARENT {
                    right.set(Rgba8::RED);
                }
            }
        });

        let after = before_cell.into_map(|cell| cell.get());
        // after
        //     .to_field(Rgba8::ZERO)
        //     .save(format!("{folder}/{name}_out.png"))
        //     .unwrap();

        let expected: Pixmap2<Rgba8> = Field::load(format!("{folder}/{name}_after.png"))
            .unwrap()
            .into();

        assert_eq!(after, expected);
    }

    #[test]
    fn mark_tile_interface_a() {
        test_mark_tile_interface("a")
    }

    #[test]
    fn mark_tile_interface_b() {
        test_mark_tile_interface("b")
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

    #[test]
    fn regions() {
        let folder = "test_resources/regions";

        let color_map: Pixmap2<Rgba8> = Field::load(format!("{folder}/b.png")).unwrap().into();

        let region_map = pixmap_regions2(&color_map);

        // convert to Rgba8
        let region_map_rgba = region_map.map(|id| Rgba8::new(*id as u8, 0, 0, 255));

        // region_map_rgba
        //     .to_field(Rgba8::ZERO)
        //     .save(format!("{folder}/b_out.png"))
        //     .unwrap();
    }
}
