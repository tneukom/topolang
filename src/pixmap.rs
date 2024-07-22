use crate::{
    area_cover::AreaCover,
    field::{Field, FieldIndex, MaterialField, RgbaField},
    material::Material,
    math::{
        interval::Interval,
        pixel::{rect_boundary_sides, Pixel, Side},
        point::Point,
        rect::Rect,
        rgba8::Rgba8,
    },
    topology::Border,
    utils::IteratorPlus,
};
use std::{
    borrow::Borrow,
    collections::BTreeMap,
    ops::{Deref, Index},
    rc::Rc,
    sync::OnceLock,
};

/// PartialEq for Rc<T: Eq> checks pointer equality first, see
/// https://github.com/rust-lang/rust/blob/ec1b69852f0c24ae833a74303800db2229b6653e/library/alloc/src/rc.rs#L2254
/// Therefore tile_a == tile_b will return true immediately if Rc::ptr_eq(tile_a, tile_b). This
/// is an important optimization for Pixmap comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tile<T> {
    pub field: Field<Option<T>>,
}

impl<T> Tile<T> {
    pub fn new() -> Self {
        let field = Field::from_map(TILE_BOUNDS, |_| None);
        Self { field }
    }

    pub fn from_field(field: Field<Option<T>>) -> Self {
        assert_eq!(field.bounds(), TILE_BOUNDS);
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

    pub fn into_map<S>(self, mut f: impl FnMut(T) -> S) -> Tile<S> {
        let field = self.field.into_map(|opt| opt.map(&mut f));
        Tile { field }
    }

    /// All items are None
    pub fn is_empty(&self) -> bool {
        self.field.iter().all(|item| item.is_none())
    }
}

impl<T: Clone> Tile<T> {
    pub fn filled(value: T) -> Self {
        let field = Field::filled(TILE_BOUNDS, Some(value));
        Self { field }
    }

    pub fn map<S>(&self, mut f: impl FnMut(&T) -> S) -> Tile<S> {
        let field = self.field.map(|value| value.as_ref().map(&mut f));
        Tile { field }
    }

    /// Does not drop tiles that become all None
    pub fn filter_map<S>(&self, mut f: impl FnMut(&T) -> Option<S>) -> Tile<S> {
        let field = self.field.map(|value| value.as_ref().and_then(&mut f));
        Tile { field }
    }

    pub fn filter(&self, mut pred: impl FnMut(&T) -> bool) -> Self {
        self.filter_map(|value| pred(value).then(|| value.clone()))
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

    pub fn blit_op<S>(&mut self, other: &Tile<S>, mut op: impl FnMut(&mut Option<T>, &S)) {
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

pub const TILE_SIZE: i64 = 64;
pub const TILE_BOUNDS: Rect<i64> =
    Rect::new(Interval::new(0, TILE_SIZE), Interval::new(0, TILE_SIZE));

/// Pixel rectangle of the given tile index
pub fn tile_rect(tile_index: Point<i64>) -> Rect<i64> {
    TILE_BOUNDS + tile_index * TILE_SIZE
}

pub fn iter_tile_pixels(tile_index: Point<i64>) -> impl IteratorPlus<Pixel> {
    tile_rect(tile_index).iter_half_open()
}

pub fn iter_zero_tile_pixels() -> impl IteratorPlus<Pixel> {
    TILE_BOUNDS.iter_half_open()
}

/// Contains interior and boundary sides
pub fn iter_sides_in_rect(rect: Rect<i64>) -> impl IteratorPlus<Side> {
    let pixel_iter = rect.iter_half_open();
    pixel_iter.flat_map(|pixel| pixel.sides_ccw().into_iter())
}

pub fn zero_tile_boundary_sides() -> &'static [Side] {
    static SIDES: OnceLock<Vec<Side>> = OnceLock::new();
    let sides = SIDES.get_or_init(|| {
        let set = rect_boundary_sides(tile_rect(Point(0, 0)));
        set.into_iter().collect()
    });
    sides.as_slice()
}

/// Contains only boundary sides
pub fn iter_tile_boundary_sides(tile_index: Point<i64>) -> impl IteratorPlus<Side> {
    let zero_tile_sides = zero_tile_boundary_sides();
    let offset = tile_index * TILE_SIZE;
    zero_tile_sides
        .iter()
        .map(move |&zero_side| zero_side + offset)
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
    // TODO: Use [[]]
    tiles: Field<Option<&'a Tile<T>>>,
}

impl<'a, T: Clone> Neighborhood<'a, T> {
    pub fn new(
        tiles: &'a BTreeMap<Point<i64>, impl Borrow<Tile<T>>>,
        tile_index: Point<i64>,
    ) -> Self {
        let bounds = Rect::low_size(tile_index - Point(1, 1), Point(3, 3));
        let tiles = Field::from_map(bounds, |tile_index| {
            tiles.get(&tile_index).map(Borrow::borrow)
        });
        Self { tile_index, tiles }
    }

    pub fn from_pixmap(pixmap: &'a Pixmap<T>, tile_index: Point<i64>) -> Self {
        Self::new(&pixmap.tiles, tile_index)
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

    /// Panics if index is out of bounds
    pub fn side_neighbors(&self, side: Side) -> SideNeighbors<&'a T> {
        let left = self.get(side.left_pixel);
        let right = self.get(side.right_pixel());
        SideNeighbors { side, left, right }
    }

    pub fn into_iter_side_neighbors(self) -> impl IteratorPlus<SideNeighbors<&'a T>> {
        self.iter_sides().map(move |side| {
            let left = self.get(side.left_pixel);
            let right = self.get(side.right_pixel());
            SideNeighbors { side, left, right }
        })
    }
}

#[derive(Debug, Clone)]
pub struct SideNeighbors<T> {
    pub side: Side,
    pub left: Option<T>,
    pub right: Option<T>,
}

#[derive(Debug, Clone)]
pub struct Pixmap<T> {
    pub tiles: BTreeMap<Point<i64>, Rc<Tile<T>>>,
}

pub type MaterialMap = Pixmap<Material>;
pub type RgbaMap = Pixmap<Rgba8>;

impl<T> Pixmap<T> {
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

    /// Iterate all entries contained in `cover`
    pub fn iter_cover<'a>(
        &'a self,
        cover: &'a AreaCover,
    ) -> impl IteratorPlus<(Point<i64>, &T)> + 'a {
        let tiles = cover
            .iter_tiles()
            .filter_map(|tile_index| self.tiles.get(&tile_index).map(|tile| (tile_index, tile)));

        tiles.flat_map(|(tile_index, tile)| {
            tile.iter()
                .map(move |(sub_index, value)| (combine_indices(tile_index, sub_index), value))
        })
    }

    /// Panics if we cannot get a mutable reference to any of the tiles (we can get a mutable
    /// reference if the ref count is one).
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Point<i64>, &mut T)> {
        self.tiles.iter_mut().flat_map(|(&tile_index, tile)| {
            // Panics if the ref count of tile is not one.
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
        crate::math::rect::RectBounds::iter_bounds(self.keys()).inc_high()
    }

    pub fn get_tile(&self, tile_index: Point<i64>) -> Option<&Tile<T>> {
        Some(self.tiles.get(&tile_index)?.as_ref())
    }

    /// Expensive, try to do bulk operations Tile by Tile
    pub fn get(&self, index: impl FieldIndex) -> Option<&T> {
        let (tile_index, pixel_index) = split_index(index.point());
        let tile = self.tiles.get(&tile_index)?.as_ref();
        tile.get(pixel_index)
    }
}

impl<T: Clone> Pixmap<T> {
    /// Per pixel methods are slow, avoid if possible
    /// Has to clone the tile that contains the given pixel if other Pixmap reference the tile.
    pub fn set(&mut self, index: impl FieldIndex, value: T) -> Option<T> {
        let (tile_index, pixel_index) = split_index(index.point());
        self.get_tile_mut(tile_index).set(pixel_index, value)
    }

    /// Per pixel methods are slow, avoid if possible
    pub fn remove(&mut self, index: impl FieldIndex) -> Option<T> {
        let (tile_index, pixel_index) = split_index(index.point());
        self.get_tile_mut(tile_index).remove(pixel_index)
    }

    // If no tile exists at the given location a new one is created.
    fn get_tile_mut(&mut self, tile_index: Point<i64>) -> &mut Tile<T> {
        let rc_tile = self
            .tiles
            .entry(tile_index)
            .or_insert_with(|| Rc::new(Tile::new()));
        Rc::make_mut(rc_tile)
    }

    pub fn map<S>(&self, mut f: impl FnMut(&T) -> S) -> Pixmap<S> {
        let tiles: BTreeMap<_, _> = self
            .tiles
            .iter()
            .map(|(&index, rc_tile)| {
                let mapped_tile = rc_tile.as_ref().map(&mut f);
                (index, Rc::new(mapped_tile))
            })
            .collect();
        Pixmap { tiles }
    }

    /// Does not drop tiles that become all None
    pub fn filter_map<S>(&self, mut f: impl FnMut(&T) -> Option<S>) -> Pixmap<S> {
        let tiles = self
            .tiles
            .iter()
            .map(|(&tile_index, tile)| {
                let tile = tile.filter_map(&mut f);
                (tile_index, Rc::new(tile))
            })
            .collect();
        Pixmap { tiles }
    }

    pub fn filter(&self, mut pred: impl FnMut(&T) -> bool) -> Self {
        self.filter_map(|value| pred(value).then(|| value.clone()))
    }

    pub fn into_map<S>(self, mut f: impl FnMut(T) -> S) -> Pixmap<S> {
        let tiles: BTreeMap<_, _> = self
            .tiles
            .into_iter()
            .map(|(index, rc_tile)| {
                let mapped_tile = Rc::unwrap_or_clone(rc_tile).into_map(&mut f);
                (index, Rc::new(mapped_tile))
            })
            .collect();
        Pixmap { tiles }
    }

    pub fn blit_if(&mut self, other: &Self, mut pred: impl FnMut(&Option<T>) -> bool) {
        for (&other_tile_index, other_tile) in &other.tiles {
            self.get_tile_mut(other_tile_index)
                .blit_if(&other_tile, &mut pred);
        }
    }

    /// Blit if there is already an existing entry.
    pub fn blit_over(&mut self, other: &Self) {
        self.blit_if(other, |current| current.is_some());
    }

    pub fn blit_op<S>(
        &mut self,
        other: &Pixmap<S>,
        cover: &AreaCover,
        mut op: impl FnMut(&mut Option<T>, &S),
    ) {
        for tile_index in cover.iter_tiles() {
            if let Some(other_tile) = other.get_tile(tile_index) {
                self.get_tile_mut(tile_index).blit_op(other_tile, &mut op);
            }
        }
    }

    pub fn blit_field(&mut self, field: &Field<T>) {
        self.modify_rect(field.bounds(), |pixel, value| {
            *value = Some(field[pixel].clone());
        });
    }

    /// Call f(pixel, &mut value) for each (pixel, value) in rect
    pub fn modify_rect(&mut self, rect: Rect<i64>, mut f: impl FnMut(Point<i64>, &mut Option<T>)) {
        for tile_index in tile_cover(rect).iter_half_open() {
            let tile = self.get_tile_mut(tile_index);

            for pixel_offset in iter_zero_tile_pixels() {
                let pixel = combine_indices(tile_index, pixel_offset);
                // TODO:SPEEDUP: Instead of checking if rect contains pixel each loop iteration
                // intersect rects before loop.
                if rect.half_open_contains(pixel) {
                    f(pixel, &mut tile.field[pixel_offset]);
                }
            }
        }
    }

    pub fn fill_rect(&mut self, rect: Rect<i64>, fill: T) {
        self.modify_rect(rect, |_, value| {
            *value = Some(fill.clone());
        });
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

    // /// Iterate tile with their neighborhoods
    // pub fn iter_neighborhoods(&self) -> impl IteratorPlus<Neighborhood<T>> {
    //     Neighborhood::iter_neighborhoods(&self.tiles)
    // }
    //
    // /// Iterate over all sides of set pixels with their left and right neighbors.
    // pub fn iter_side_neighbors(&self) -> impl Iterator<Item = SideNeighbors<&T>> {
    //     Neighborhood::iter_side_neighbors(&self.tiles)
    // }
    //
    // pub fn iter_tile_interface_sides(&self) -> impl Iterator<Item = SideNeighbors<&T>> {
    //     Neighborhood::iter_tile_interface_sides(&self.tiles)
    // }

    /// Iterate tile with their neighborhoods
    pub fn iter_neighborhoods(&self) -> impl IteratorPlus<Neighborhood<T>> {
        self.tiles
            .keys()
            .map(|&tile_index| Neighborhood::from_pixmap(self, tile_index))
    }

    /// Iterate over all sides of set pixels with their left and right neighbors.
    pub fn iter_side_neighbors(&self) -> impl Iterator<Item = SideNeighbors<&T>> {
        self.iter_neighborhoods()
            .flat_map(|neighborhood| neighborhood.into_iter_side_neighbors())
    }

    pub fn iter_tile_interface_sides(&self) -> impl Iterator<Item = SideNeighbors<&T>> {
        self.tiles.keys().flat_map(|&tile_index| {
            let neighborhood = Neighborhood::from_pixmap(self, tile_index);
            // TODO: iter_tile_boundary_sides uses a global variable therefore atomic compare
            iter_tile_boundary_sides(tile_index).map(move |side| neighborhood.side_neighbors(side))
        })
    }

    pub fn translated(&self, offset: Point<i64>) -> Self {
        let mut result = Self::new();
        for (&tile_index, tile) in &self.tiles {
            // Blit tile.field at offset
            let offset_tile_rect = tile_rect(tile_index) + offset;
            result.modify_rect(offset_tile_rect, |pixel, value| {
                *value = tile.field[pixel - offset_tile_rect.low()].clone();
            });
        }
        result
    }

    pub fn sub(&self, pixels: impl IntoIterator<Item = Pixel>) -> Self {
        let mut result = Self::new();
        for pixel in pixels {
            if let Some(value) = self.get(pixel) {
                result.set(pixel, value.clone());
            }
        }
        result
    }

    /// Extract the pixel in the interior of the given rectangle.
    pub fn sub_rect(&self, rect: Rect<i64>) -> Self {
        // TODO:SPEEDUP: Per tile
        // let result = Self::new();
        // for tile_index in tile_cover(rect).iter_half_open() {
        //     if let Some(tile) = self.tiles.get(&tile_index) {
        //         let sub_tile = tile.filter_map()
        //         result.tiles.set
        //     }
        // }

        // TODO:SPEEDUP: Could be done much faster especially for large rectangle that contain
        //   few pixels from self.
        self.sub(rect.iter_half_open())
    }

    pub fn extract(&mut self, pixels: impl IntoIterator<Item = Pixel>) -> Self {
        let mut result = Self::new();
        for pixel in pixels {
            if let Some(removed) = self.remove(pixel) {
                result.set(pixel, removed);
            }
        }
        result
    }

    /// Returns sub pixmap with the keys right of the given border.
    pub fn right_of_border(&self, border: &Border) -> Self {
        self.sub(border.right_pixels())
    }

    /// Extract the pixels on the right side of the given border.
    /// TODO: Use border.bounds(), skip BTreeSet
    /// TODO:REMOVE should use right_of_border
    pub fn extract_right_of_border(&mut self, border: &Border) -> Self {
        self.extract(border.right_pixels())
    }
}

impl<T: Eq + Clone> Pixmap<T> {
    /// TODO: Rename to something more generic, like without_value, drop_value, reject_value
    pub fn without(&self, removed: &T) -> Self {
        self.filter(|value| value != removed)
    }
}

impl<T: PartialEq + Clone> PartialEq for Pixmap<T> {
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

impl<Idx: FieldIndex, T> Index<Idx> for Pixmap<T> {
    type Output = T;

    fn index(&self, pixel: Idx) -> &Self::Output {
        self.get(pixel).unwrap()
    }
}

impl Pixmap<Rgba8> {
    pub fn into_material(self) -> Pixmap<Material> {
        self.into_map(|rgba| Material::from(rgba))
    }
}

impl Pixmap<Material> {
    pub fn into_rgba8(self) -> Pixmap<Rgba8> {
        self.into_map(|material| material.color())
    }
}

impl From<&RgbaField> for RgbaMap {
    fn from(field: &RgbaField) -> Self {
        let mut pixmap = Self::new();
        pixmap.blit_field(field);
        pixmap
    }
}

impl From<RgbaField> for RgbaMap {
    fn from(field: RgbaField) -> Self {
        Self::from(&field)
    }
}

impl From<&MaterialField> for MaterialMap {
    fn from(field: &MaterialField) -> Self {
        Self::from_field(field)
    }
}

impl From<MaterialField> for MaterialMap {
    fn from(field: MaterialField) -> Self {
        Self::from(&field)
    }
}

impl From<&RgbaField> for MaterialMap {
    fn from(field: &RgbaField) -> Self {
        let rgba_map: RgbaMap = field.into();
        // Since Rgba8 and Material have the same size, allocated memory in Tiles should be reused.
        rgba_map.into_material()
    }
}

impl From<RgbaField> for MaterialMap {
    fn from(field: RgbaField) -> Self {
        Self::from(&field)
    }
}
