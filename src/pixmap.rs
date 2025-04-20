use crate::{
    field::{Field, FieldIndex, MaterialField, RgbaField},
    material::Material,
    math::{pixel::Side, point::Point, rect::Rect, rgba8::Rgba8},
    regions::{
        area_left_of_boundary, area_left_of_boundary_bounds, area_right_of_boundary
        ,
    },
    topology::Border,
};
use ahash::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pixmap<T> {
    pub field: Field<Option<T>>,
}

pub type MaterialMap = Pixmap<Material>;
pub type RgbaMap = Pixmap<Rgba8>;

impl<T: Copy> Pixmap<T> {
    pub fn new(field: Field<Option<T>>) -> Self {
        Self { field }
    }

    pub fn nones(bounds: Rect<i64>) -> Self {
        Self {
            field: Field::filled(bounds, None),
        }
    }

    pub fn nones_like(like: &Self) -> Self {
        Self::nones(like.bounding_rect())
    }

    pub fn filled(bounds: Rect<i64>, value: T) -> Self {
        Self {
            field: Field::filled(bounds, Some(value)),
        }
    }

    pub fn shrink(self) -> Self {
        let shrink_bounds = Rect::index_bounds(self.keys());
        let field = self.field.sub(shrink_bounds);
        Self { field }
    }

    /// Iterate over all (pixel, &value)
    pub fn iter(&self) -> impl Iterator<Item = (Point<i64>, T)> + Clone + '_ {
        self.field
            .enumerate()
            .filter_map(|(pixel, &value)| match value {
                Some(value) => Some((pixel, value)),
                None => None,
            })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Point<i64>, &mut T)> + '_ {
        self.field
            .enumerate_mut()
            .filter_map(|(pixel, value)| match value {
                Some(value) => Some((pixel, value)),
                None => None,
            })
    }

    /// Iterate over all keys
    pub fn keys(&self) -> impl Iterator<Item = Point<i64>> + Clone + '_ {
        self.iter().map(|(index, _)| index)
    }

    /// Iterate over all values
    pub fn values(&self) -> impl Iterator<Item = T> + Clone + '_ {
        self.iter().map(|(_, value)| value)
    }

    pub fn bounding_rect(&self) -> Rect<i64> {
        self.field.bounds()
    }

    /// Expensive, try to do bulk operations Tile by Tile
    pub fn get(&self, index: impl FieldIndex) -> Option<T> {
        *self.field.get(index)?
    }

    pub fn set(&mut self, index: impl FieldIndex, value: T) -> Option<T> {
        self.field.set(index, Some(value))
    }

    /// Per pixel methods are slow, avoid if possible
    pub fn remove(&mut self, index: impl FieldIndex) -> Option<T> {
        self.field.set(index, None)
    }

    pub fn put(&mut self, index: impl FieldIndex, value: Option<T>) -> Option<T> {
        self.field.set(index, value)
    }

    pub fn map<S>(&self, mut f: impl FnMut(T) -> S) -> Pixmap<S> {
        let field = self.field.map(|&value| value.map(&mut f));
        Pixmap { field }
    }

    /// Does not drop tiles that become all None
    pub fn filter_map<S>(&self, mut f: impl FnMut(Point<i64>, T) -> Option<S>) -> Pixmap<S> {
        let field = self
            .field
            .map_with_index(|pixel, &value| value.and_then(|value| f(pixel, value)));
        Pixmap { field }
    }

    pub fn filter(&self, mut pred: impl FnMut(Point<i64>, T) -> bool) -> Self {
        self.filter_map(|index, value| pred(index, value).then(|| value.clone()))
    }

    /// In some cases Rust should be able to reuse the memory of field
    pub fn into_map<S>(self, mut f: impl FnMut(T) -> S) -> Pixmap<S> {
        let field = self.field.into_map(|value| value.map(&mut f));
        Pixmap { field }
    }

    /// Set `self[pixel]` to `other[pixel]` if `other[pixel]` is Some.
    pub fn blit(&mut self, other: &Self) {
        let rect = self.bounding_rect().intersect(other.bounding_rect());
        for pixel in rect.iter_indices() {
            if let Some(value) = other.get(pixel) {
                self.set(pixel, value);
            }
        }
    }

    pub fn fill_rect(&mut self, rect: Rect<i64>, fill: T) {
        for pixel in rect.intersect(self.bounding_rect()).iter_indices() {
            self.set(pixel, fill);
        }
    }

    pub fn from_field(field: &Field<T>) -> Self {
        let field = field.map(|&value| Some(value));
        Self { field }
    }

    pub fn to_field(&self, default: T) -> Field<T> {
        self.field
            .map(|value| value.clone().unwrap_or(default.clone()))
    }

    pub fn translated(self, offset: Point<i64>) -> Self {
        Self {
            field: self.field.translated(offset),
        }
    }

    pub fn left_of_boundary(&self, sides: impl Iterator<Item = Side> + Clone) -> Self {
        let bounds = area_left_of_boundary_bounds(sides.clone());
        let mut field = Field::filled(bounds, None);
        for pixel in area_left_of_boundary(sides) {
            field.set(pixel, self.field[pixel].clone());
        }
        Self { field }
    }

    pub fn right_of_boundary(&self, sides: impl Iterator<Item = Side> + Clone) -> Self {
        self.left_of_boundary(sides.map(Side::reversed))
    }

    pub fn left_of_border(&self, border: &Border) -> Self {
        self.left_of_boundary(border.sides())
    }

    /// Returns sub pixmap with the keys right of the given border.
    pub fn right_of_border(&self, border: &Border) -> Self {
        self.right_of_boundary(border.sides())
    }

    pub fn fill_right_of_border(&mut self, border: &Border, value: T) {
        for pixel in area_right_of_boundary(border.sides()) {
            self.set(pixel, value);
        }
    }

    pub fn integer_upscale(&self, scale: i64) -> Self {
        Self {
            field: self.field.integer_upscale(scale),
        }
    }
}

impl<T: Copy + Eq> Pixmap<T> {
    /// Iterate all `pixel` in `cover` where `self[pixel] == value`
    pub fn iter_where_value<'a>(
        &'a self,
        bounds: Rect<i64>,
        value: T,
    ) -> impl Iterator<Item = Point<i64>> + Clone + 'a {
        // TODO: Intersect `self.bounds` and `bounds`
        bounds
            .iter_indices()
            .filter(move |&pixel| self.get(pixel) == Some(value))
    }

    /// TODO: Rename to something more generic, like without_value, drop_value, reject_value
    pub fn without(&self, removed: T) -> Self {
        self.filter(|_, value| value != removed)
    }

    /// Is `self[pixel] == other[pixel]` where `self[pixel]` is defined.
    pub fn defined_subset_of(&self, other: &Self) -> bool {
        self.iter()
            .all(|(pixel, value)| Some(value) == other.get(pixel))
    }

    // Is `self[pixel] == other[pixel]` for all `pixel`. Implies that the domain of definition
    // of `self` and `other` are the same, but not that their bounds are equal.
    pub fn defined_equals(&self, other: &Self) -> bool {
        self.defined_subset_of(other) && other.defined_subset_of(self)
    }
}

impl Pixmap<Rgba8> {
    pub fn into_material(self) -> Pixmap<Material> {
        self.into_map(|rgba| Material::from(rgba))
    }

    pub fn save(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        self.to_field(Material::UNDEF_COLOR).save(path)
    }
}

impl Pixmap<Material> {
    pub fn into_rgba(self) -> Pixmap<Rgba8> {
        self.into_map(|material| material.to_rgba())
    }

    pub fn to_rgba_field(&self, default: Material) -> Field<Rgba8> {
        self.field
            .map(|material| material.unwrap_or(default).to_rgba())
    }

    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let rgba_field = RgbaField::load(path)?;
        Ok(MaterialMap::from(rgba_field))
    }

    pub fn save(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        self.clone().into_rgba().save(path)
    }
}

impl From<&RgbaField> for RgbaMap {
    fn from(field: &RgbaField) -> Self {
        Self::from_field(field)
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

impl<T: Copy> From<&HashMap<Point<i64>, T>> for Pixmap<T> {
    fn from(hash_map: &HashMap<Point<i64>, T>) -> Self {
        let bounds = Rect::index_bounds(hash_map.keys().copied());
        let mut pixmap = Pixmap::nones(bounds);
        for (&index, &value) in hash_map {
            pixmap.set(index, value);
        }
        pixmap
    }
}
