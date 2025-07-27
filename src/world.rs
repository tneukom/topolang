use crate::{
    field::RgbaField,
    frozen::{Frozen, FrozenBoundary},
    material::Material,
    material_effects::paint_material_map_effects,
    math::{
        pixel::{Pixel, Side},
        rect::Rect,
        rgba8::Rgba8,
    },
    pixmap::MaterialMap,
    regions::split_boundary_into_cycles,
    topology::{RegionKey, Topology},
    view::Selection,
};
use ahash::HashSet;
use std::{
    cell::{Cell, RefCell},
    path::Path,
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone)]
struct CachedRgbaField {
    rgba_field: Arc<RwLock<RgbaField>>,

    /// Where `rgba_field` is not fresh anymore and needs to be recomputed.
    expired_bounds: Cell<Rect<i64>>,
}

impl CachedRgbaField {
    fn new(bounds: Rect<i64>) -> Self {
        let rgba_field = RgbaField::filled(bounds, Rgba8::TRANSPARENT);
        Self {
            rgba_field: Arc::new(RwLock::new(rgba_field)),
            expired_bounds: Cell::new(bounds),
        }
    }

    fn expire_rgba_rect(&self, rect: Rect<i64>) {
        // Some effects (before, after material) depend on neighboring pixels.
        let padded = rect.padded(1);
        let expanded = self.expired_bounds.get().bounds_with_rect(padded);
        self.expired_bounds.set(expanded);
    }

    /// Recomputes stale areas of rgba_field, so can be expensive
    fn fresh_rgba_field(&self, material_map: &MaterialMap) -> (Arc<RwLock<RgbaField>>, Rect<i64>) {
        let expired_bounds = self.expired_bounds.get();
        if !expired_bounds.is_empty() {
            let mut write_rgba_field = self.rgba_field.write().unwrap();
            paint_material_map_effects(material_map, &mut write_rgba_field);
            self.expired_bounds.set(Rect::EMPTY);
        }
        (self.rgba_field.clone(), expired_bounds)
    }
}

/// Sides where left is solid and right is not.
pub fn solid_boundary_sides(topology: &Topology) -> HashSet<Side> {
    let mut sides = HashSet::default();
    for region in topology.regions.values() {
        if region.material.is_solid() {
            for side in region.boundary.iter_sides() {
                // side cancels out side.reverse()
                if !sides.remove(&side.reversed()) {
                    sides.insert(side);
                }
            }
        }
    }
    sides
}

/// BottomRight and TopLeft sides are discarded
pub fn solid_boundary_cycles(topology: &Topology) -> Vec<Vec<Side>> {
    let sides = solid_boundary_sides(topology);
    split_boundary_into_cycles(sides)
}

#[derive(Debug, Clone)]
pub struct World {
    material_map: MaterialMap,

    /// None means the topology has to be recomputed from the material_map
    topology: Topology,

    rgba_field: CachedRgbaField,

    solid_boundary: RefCell<FrozenBoundary>,
}

impl World {
    pub fn from_material_map(material_map: MaterialMap) -> Self {
        Self {
            rgba_field: CachedRgbaField::new(material_map.bounding_rect()),
            topology: Topology::new(&material_map),
            solid_boundary: RefCell::new(Frozen::invalid(Default::default())),
            material_map,
        }
    }

    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let material_map = MaterialMap::load(path)?;
        let world = Self::from_material_map(material_map);
        Ok(world)
    }

    pub fn topology(&self) -> &Topology {
        &self.topology
    }

    pub fn material_map(&self) -> &MaterialMap {
        &self.material_map
    }

    pub fn bounds(&self) -> Rect<i64> {
        self.material_map.bounding_rect()
    }

    /// Returns true if the region was modified, it did not already have the assigned material.
    #[inline(never)]
    pub fn fill_region(&mut self, region_key: RegionKey, material: Material) -> bool {
        let region = &self.topology[region_key];
        if region.material == material {
            return false;
        }

        let region_area = region.boundary.interior_area();
        for &pixel in &region_area {
            self.material_map.set(pixel, material);
        }

        self.rgba_field.expire_rgba_rect(region.bounds());

        if !self.topology.try_set_region_material(region_key, material) {
            self.topology
                .update(&self.material_map, region_area.into_iter());
        }

        true
    }

    /// Returns true if any pixels material was changed.
    #[inline(never)]
    pub fn draw(&mut self, pixel_materials: impl Iterator<Item = (Pixel, Material)>) -> bool {
        let mut changed_pixels = Vec::new();
        for (pixel, material) in pixel_materials {
            let previous_material = self.material_map.set(pixel, material);
            if previous_material != Some(material) {
                changed_pixels.push(pixel);
            }
        }

        if changed_pixels.is_empty() {
            return false;
        }

        let draw_bounds = self
            .topology
            .update(&self.material_map, changed_pixels.into_iter());

        self.rgba_field.expire_rgba_rect(draw_bounds);

        true
    }

    /// Blit passed Pixmap to self.material_map but only where material_map is already defined.
    pub fn blit(&mut self, other: &MaterialMap) {
        self.material_map.blit(other);
        self.topology
            .update(&self.material_map, other.bounding_rect().iter_indices());
        self.rgba_field.expire_rgba_rect(other.bounding_rect());
    }

    pub fn rect_selection(&mut self, rect: Rect<i64>) -> Selection {
        let rect = rect.intersect(self.material_map.bounding_rect());
        let mut selection = MaterialMap::nones(rect);
        for pixel in rect.iter_indices() {
            selection.put(pixel, self.material_map.set(pixel, Material::TRANSPARENT));
        }

        self.topology
            .update(&self.material_map, rect.iter_indices());

        self.rgba_field.expire_rgba_rect(rect);
        Selection::new(selection)
    }

    pub fn region_selection(&mut self, region_key: RegionKey) -> Selection {
        let region = &self.topology[region_key];
        let bounds = region.bounds();
        let region_area = region.boundary.interior_area();

        let mut selection = MaterialMap::nones(bounds);
        for &pixel in &region_area {
            selection.put(pixel, self.material_map.set(pixel, Material::TRANSPARENT));
        }

        self.topology
            .update(&self.material_map, region_area.into_iter());

        self.rgba_field.expire_rgba_rect(bounds);
        Selection::new(selection)
    }

    /// Recomputes stale areas of rgba_field, so can be expensive
    pub fn fresh_rgba_field(&self) -> (Arc<RwLock<RgbaField>>, Rect<i64>) {
        self.rgba_field.fresh_rgba_field(&self.material_map)
    }

    pub fn solid_boundary(&self) -> FrozenBoundary {
        let mut solid_boundary = self.solid_boundary.borrow_mut();
        solid_boundary.update_boundary(&self.topology);
        solid_boundary.clone()
    }

    // pub fn rgba_field(&self) -> &RgbaField {
    //     self.rgba_field.get_or_init(|| {
    //         // Use MaterialMap::effects_rgba instead
    //         self.material_map.to_rgba_field(Material::TRANSPARENT)
    //     })
    // }

    // pub fn update_effects(&mut self) {
    //     if self.effects_dirty.is_empty() {
    //         return;
    //     }
    //
    //     let topology = self.topology.get_or_init(&self.material_map);
    //
    //     let padded_effects_dirty = self.effects_dirty.padded(1);
    //
    //     for (&region_key, region) in &topology.regions {
    //         if region.bounds().intersects(padded_effects_dirty) {
    //             apply_region_material_effect(&mut self.material_map, topology, region_key, region);
    //         }
    //     }
    //
    //     self.effects_dirty = Rect::EMPTY;
    // }
}

impl From<MaterialMap> for World {
    fn from(material_map: MaterialMap) -> Self {
        Self::from_material_map(material_map)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        field::RgbaField, material::Material, pixmap::MaterialMap, utils::IntoT, world::World,
    };

    fn assert_blit(name: &str) {
        let folder = format!("test_resources/topology/blit/{name}");

        let world_pixmap = RgbaField::load(format!("{folder}/base.png"))
            .unwrap()
            .intot::<MaterialMap>();

        let blit = RgbaField::load(format!("{folder}/blit.png"))
            .unwrap()
            .intot::<MaterialMap>()
            .without(Material::TRANSPARENT);

        let mut expected_pixmap = world_pixmap.clone();
        expected_pixmap.blit(&blit);
        let expected_world = World::from_material_map(expected_pixmap);

        let mut world = World::from_material_map(world_pixmap);
        world.blit(&blit);

        assert_eq!(world.material_map(), expected_world.material_map());
        assert_eq!(world.topology(), expected_world.topology());
    }

    #[test]
    fn blit_a() {
        assert_blit("a");
    }

    #[test]
    fn blit_b() {
        assert_blit("b");
    }

    #[test]
    fn blit_c() {
        assert_blit("c");
    }

    #[test]
    fn blit_d() {
        assert_blit("d");
    }

    #[test]
    fn blit_e() {
        assert_blit("e");
    }

    #[test]
    fn blit_f() {
        assert_blit("f");
    }
}
