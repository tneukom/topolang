use std::{collections::HashMap, ops::Index, path::Path, rc::Rc};

use crate::{
    area_cover::AreaCover,
    connected_components::right_of_border,
    field::{Field, FieldIndex, RgbaField},
    material::Material,
    math::{
        generic::EuclidDivRem,
        interval::Interval,
        pixel::Pixel,
        point::Point,
        rect::{Rect, RectBounds},
        rgba8::Rgba8,
    },
    topology::Border,
    utils::IteratorPlus,
};

/// PartialEq for Rc<T: Eq> checks pointer equality first, see
/// https://github.com/rust-lang/rust/blob/ec1b69852f0c24ae833a74303800db2229b6653e/library/alloc/src/rc.rs#L2254
/// Therefore tile_a == tile_b will return true immediately if Rc::ptr_eq(tile_a, tile_b). This
/// is an important optimization for Pixmap comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tile<T> {
    field: Field<Option<T>>,
}

impl<T> Tile<T> {
    const SIZE: i64 = 64;
    const BOUNDS: Rect<i64> = Rect::new(Interval::new(0, Self::SIZE), Interval::new(0, Self::SIZE));

    pub fn iter<'a>(&'a self) -> impl IteratorPlus<(Point<i64>, &T)> + 'a {
        self.field
            .enumerate()
            .filter_map(|(index, opt_value)| opt_value.as_ref().map(|value| (index, value)))
    }

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
}

impl<T: Clone> Tile<T> {
    pub fn new() -> Self {
        let field = Field::filled(Self::BOUNDS, None);
        Self { field }
    }

    pub fn filled(value: T) -> Self {
        let field = Field::filled(Self::BOUNDS, Some(value));
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

#[derive(Debug, Clone, Eq)]
pub struct Pixmap<T> {
    tiles: Field<Option<Rc<Tile<T>>>>,
}

pub type PixmapRgba = Pixmap<Rgba8>;
pub type PixmapMaterial = Pixmap<Material>;

impl<T> Pixmap<T> {
    const ROWS: i64 = 64;
    const COLUMNS: i64 = 64;
    const BOUNDS: Rect<i64> = Rect::new(
        Interval::new(0, Self::COLUMNS),
        Interval::new(0, Self::ROWS),
    );

    /// Pixel rectangle of the given tile index
    pub fn tile_rect(tile_index: Point<i64>) -> Rect<i64> {
        Tile::<T>::BOUNDS + tile_index * Tile::<T>::SIZE
    }

    /// Split pixel index into tile and pixel index
    pub fn split_index(index: Point<i64>) -> (Point<i64>, Point<i64>) {
        let tile_index = index.euclid_div(Tile::<T>::SIZE);
        let pixel_index = index.euclid_rem(Tile::<T>::SIZE);
        (tile_index, pixel_index)
    }

    pub fn combine_indices(tile_index: Point<i64>, pixel_index: Point<i64>) -> Point<i64> {
        tile_index * Tile::<T>::SIZE + pixel_index
    }

    /// Iterate over the valid tile indices
    pub fn tile_indices() -> impl IteratorPlus<Point<i64>> {
        Self::BOUNDS.iter_half_open()
    }

    pub fn get_tile(&self, tile_index: Point<i64>) -> Option<&Tile<T>> {
        let rc_tile = self.tiles.get(tile_index).unwrap().as_ref()?;
        Some(rc_tile.as_ref())
    }

    pub fn get_rc_tile(&self, tile_index: Point<i64>) -> &Option<Rc<Tile<T>>> {
        self.tiles.get(tile_index).unwrap()
    }

    pub fn get(&self, index: impl FieldIndex) -> Option<&T> {
        let (tile_index, pixel_index) = Self::split_index(index.point());
        let tile = self.tiles.get(tile_index)?.as_ref()?;
        tile.get(pixel_index)
    }

    fn enumerate_tiles<'a>(&'a self) -> impl IteratorPlus<(Point<i64>, &Tile<T>)> + 'a {
        self.tiles.enumerate().flat_map(|(tile_index, opt_tile)| {
            opt_tile.as_ref().map(|tile| (tile_index, tile.as_ref()))
        })
    }

    fn iter_from_tiles<'a>(
        &'a self,
        tiles: impl IteratorPlus<(Point<i64>, &'a Tile<T>)> + 'a,
    ) -> impl IteratorPlus<(Point<i64>, &T)> + 'a {
        tiles.flat_map(|(tile_index, tile)| {
            tile.iter().map(move |(pixel_index, value)| {
                (Self::combine_indices(tile_index, pixel_index), value)
            })
        })
    }

    pub fn iter<'a>(&'a self) -> impl IteratorPlus<(Point<i64>, &T)> + 'a {
        self.iter_from_tiles(self.enumerate_tiles())
    }

    pub fn iter_cover<'a>(
        &'a self,
        cover: &'a AreaCover,
    ) -> impl IteratorPlus<(Point<i64>, &T)> + 'a {
        self.iter_from_tiles(self.enumerate_cover_tiles(cover))
    }

    pub fn enumerate_cover_tiles<'a>(
        &'a self,
        cover: &'a AreaCover,
    ) -> impl IteratorPlus<(Point<i64>, &Tile<T>)> + 'a {
        cover
            .iter_tiles()
            .filter_map(|tile_index| self.get_tile(tile_index).map(|tile| (tile_index, tile)))
    }

    pub fn keys<'a>(&'a self) -> impl IteratorPlus<Point<i64>> + 'a {
        self.iter().map(|kv| kv.0)
    }

    pub fn values<'a>(&'a self) -> impl IteratorPlus<&T> + 'a {
        self.iter().map(|kv| kv.1)
    }

    pub fn bounding_rect(&self) -> Rect<i64> {
        // TODO: In many cases returning the bounding rectangle of the tiles would be enough
        // TODO:SPEEDUP: Should be cached!
        RectBounds::iter_bounds(self.keys()).inc_high()
    }

    // pub fn map<S>(&self, mut f: impl FnMut(&T) -> S) -> Pixmap<S> {
    //     let tiles = self
    //         .tiles
    //         .map(|opt_tile| opt_tile.as_ref().map(|tile| Rc::new(tile.map(&mut f))));
    //     Pixmap { tiles }
    // }
    //
    // pub fn map_into<S: From<T>>(self) -> Pixmap<S> {
    //     Pixmap {
    //         tiles: self.tiles.map_into(),
    //     }
    // }
}

impl<T: Clone> Pixmap<T> {
    pub fn new() -> Self {
        let tiles = Field::filled(Self::BOUNDS, None);
        Self { tiles }
    }

    pub fn filled(value: T) -> Self {
        let tile = Rc::new(Tile::filled(value));
        let tiles = Field::filled(Self::BOUNDS, Some(tile));
        Self { tiles }
    }

    // pub fn get_mut(&mut self, index: impl FieldIndex) -> Option<&mut T> {
    //     let (tile_index, pixel_index) = Self::split_index(index.point());
    //     let tile = self.tiles.get_mut(tile_index)?.as_ref()?;
    //     tile.pixels.get_mut(pixel_index).unwrap()
    // }

    /// Panics if tile_index is outside of tile_index_bounds
    fn get_tile_mut(&mut self, tile_index: Point<i64>) -> &mut Tile<T> {
        let opt_rc_tile = self.tiles.get_mut(tile_index).unwrap();
        let rc_tile = opt_rc_tile.get_or_insert_with(|| Rc::new(Tile::new()));
        Rc::make_mut(rc_tile)
    }

    /// Has to clone the tile that contains the given pixel if other Pixmap reference the tile
    pub fn set(&mut self, index: impl FieldIndex, value: T) -> Option<T> {
        let (tile_index, pixel_index) = Self::split_index(index.point());
        self.get_tile_mut(tile_index).set(pixel_index, value)
    }

    pub fn remove(&mut self, index: impl FieldIndex) -> Option<T> {
        let (tile_index, pixel_index) = Self::split_index(index.point());
        self.get_tile_mut(tile_index).remove(pixel_index)
    }

    pub fn map<S>(&self, mut f: impl FnMut(&T) -> S) -> Pixmap<S> {
        let tiles = self
            .tiles
            .map(|opt_tile| opt_tile.as_ref().map(|tile| Rc::new(tile.map(&mut f))));
        Pixmap { tiles }
    }

    pub fn into_map<S>(self, mut f: impl FnMut(T) -> S) -> Pixmap<S> {
        let tiles = self.tiles.into_map(|opt_tile| {
            opt_tile.map(|rc_tile| {
                let mapped_tile = Rc::unwrap_or_clone(rc_tile).into_map(&mut f);
                Rc::new(mapped_tile)
            })
        });
        Pixmap { tiles }
    }

    /// Does not drop tiles that become all None
    pub fn filter_map<S>(&self, mut f: impl FnMut(&T) -> Option<S>) -> Pixmap<S> {
        let tiles = self.tiles.map(|opt_tile| {
            opt_tile
                .as_ref()
                .map(|tile| Rc::new(tile.filter_map(&mut f)))
        });
        Pixmap { tiles }
    }

    pub fn filter(&self, mut pred: impl FnMut(&T) -> bool) -> Self {
        self.filter_map(|value| {
            if pred(value) {
                Some(value.clone())
            } else {
                None
            }
        })
    }

    /// SPEEDUP: Could be done better tile by tile
    pub fn translated(self, offset: Point<i64>) -> Self {
        let mut result = Self::new();
        for (index, value) in self.iter() {
            result.set(index + offset, value.clone());
        }
        result
    }

    pub fn from_hashmap(pixels: &HashMap<Pixel, T>) -> Self {
        let mut result = Self::new();
        for (&pixel, value) in pixels {
            result.set(pixel, value.clone());
        }
        result
    }

    pub fn right_of_border(&self, border: &Border) -> Pixmap<T> {
        // Extract pixels left of inner_border
        let right_pixels = right_of_border(&border.cycle);
        let mut result = Self::new();
        for pixel in right_pixels {
            if let Some(value) = self.get(pixel) {
                result.set(pixel, value.clone());
            }
        }
        result
    }

    /// Entries right of boundary are removed
    /// TODO: Use border.bounds(), skip BTreeSet
    /// TODO:REMOVE should use right_of_border
    pub fn extract_right(&mut self, boundary: &Border) -> Pixmap<T> {
        // Extract pixels left of inner_border
        let right_pixels = boundary.right_pixels();
        let mut result = Self::new();
        for pixel in right_pixels {
            if let Some(removed) = self.remove(pixel) {
                result.set(pixel, removed);
            }
        }
        result
    }

    pub fn blit_if(&mut self, other: &Self, mut pred: impl FnMut(&Option<T>) -> bool) {
        for (other_tile_index, other_tile) in other.enumerate_tiles() {
            self.get_tile_mut(other_tile_index)
                .blit_if(other_tile, &mut pred);
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

    /// TODO:SPEEDUP: Tile by tile
    pub fn blit_field(&mut self, other: &Field<T>) {
        for (index, value) in other.enumerate() {
            self.set(index, value.clone());
        }
    }

    pub fn blit_to_field(&self, field: &mut Field<T>) {
        for (index, value) in self.iter() {
            field[index] = value.clone();
        }
    }

    pub fn tile_ptr_eq(lhs: &Option<Rc<Tile<T>>>, rhs: &Option<Rc<Tile<T>>>) -> bool {
        match (lhs, rhs) {
            (None, None) => true,
            (Some(rc_lhs), Some(rc_rhs)) => Rc::ptr_eq(rc_lhs, rc_rhs),
            _ => false,
        }
    }

    /// Translate pixmap to positive coordinates, top left pixel will be at (0, 0) and convert
    /// to bitmap.
    pub fn to_field(&self, default: T) -> Field<T> {
        let bounds = self.bounding_rect();
        let mut field = Field::filled(bounds, default);
        for (index, value) in self.iter() {
            field[index] = value.clone();
        }
        field
    }

    pub fn from_field(field: &Field<T>) -> Self {
        let mut pixmap = Self::new();
        pixmap.blit_field(field);
        pixmap
    }

    // pub fn shrink(&self) -> Self {
    //     let bounds = self.actual_bounds();
    //     Pixmap {
    //         field: self.field.sub(bounds),
    //     }
    // }
}

impl Pixmap<Rgba8> {
    pub fn into_material(self) -> Pixmap<Material> {
        self.into_map(|rgba| Material::from(rgba))
    }
}

impl Pixmap<Material> {
    pub fn into_rgba8(self) -> Pixmap<Rgba8> {
        self.into_map(|material| material.rgba())
    }

    pub fn load_from_memory(memory: &[u8]) -> anyhow::Result<Self> {
        Ok(RgbaField::load_from_memory(memory)?
            .into_material()
            .to_pixmap())
    }

    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        Ok(RgbaField::load(path)?.into_material().to_pixmap())
    }
}

impl<T: Eq + Clone> Pixmap<T> {
    /// TODO: Rename to something more generic, like without_value, drop_value, reject_value
    pub fn without(&self, removed: &T) -> Self {
        self.filter(|value| value != removed)
    }
}

impl<T: PartialEq> PartialEq for Pixmap<T> {
    /// SPEEDUP: Implement tile by tile
    fn eq(&self, other: &Self) -> bool {
        self.values().eq(other.values())
    }
}

impl<Idx: FieldIndex, T> Index<Idx> for Pixmap<T> {
    type Output = T;

    fn index(&self, pixel: Idx) -> &Self::Output {
        self.get(pixel).unwrap()
    }
}
