use crate::{
    field::RgbaField,
    material::Material,
    material_effects::material_map_effects,
    math::{rect::Rect, rgba8::Rgba8},
    pixmap::MaterialMap,
    topology::{FillRegion, RegionKey, Topology},
    view::Selection,
};
use std::{
    cell::{Cell, OnceCell},
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone)]
struct CachedTopology {
    topology: OnceCell<Topology>,
}

impl CachedTopology {
    pub fn empty() -> Self {
        Self {
            topology: OnceCell::new(),
        }
    }

    pub fn from(material_map: &MaterialMap) -> Self {
        let topology = Topology::new(material_map);
        Self {
            topology: OnceCell::from(topology),
        }
    }

    pub fn get_or_init(&self, material_map: &MaterialMap) -> &Topology {
        self.topology.get_or_init(|| Topology::new(material_map))
    }

    pub fn get(&self) -> Option<&Topology> {
        self.topology.get()
    }

    pub fn get_mut(&mut self) -> Option<&mut Topology> {
        self.topology.get_mut()
    }
}

#[derive(Debug, Clone)]
struct CachedRgbaField {
    rgba_field: Arc<RwLock<RgbaField>>,

    /// Where `rgba_field` is not fresh anymore and needs to be recomputed.
    expired_rgba_field: Cell<Rect<i64>>,
}

impl CachedRgbaField {
    fn new(bounds: Rect<i64>) -> Self {
        let rgba_field = RgbaField::filled(bounds, Rgba8::TRANSPARENT);
        Self {
            rgba_field: Arc::new(RwLock::new(rgba_field)),
            expired_rgba_field: Cell::new(bounds),
        }
    }

    fn expire_rgba_rect(&self, rect: Rect<i64>) {
        let expanded = self.expired_rgba_field.get().bounds_with_rect(rect);
        self.expired_rgba_field.set(expanded);
    }

    /// Recomputes stale areas of rgba_field, so can be expensive
    fn fresh_rgba_field(&self, material_map: &MaterialMap) -> Arc<RwLock<RgbaField>> {
        if !self.expired_rgba_field.get().is_empty() {
            let mut write_rgba_field = self.rgba_field.write().unwrap();
            material_map_effects(material_map, &mut write_rgba_field);
            self.expired_rgba_field.set(Rect::EMPTY);
        }
        self.rgba_field.clone()
    }
}

#[derive(Debug, Clone)]
pub struct World {
    material_map: MaterialMap,

    /// None means the topology has to be recomputed from the material_map
    topology: CachedTopology,

    rgba_field: CachedRgbaField,
}

impl World {
    pub fn from_material_map(material_map: MaterialMap) -> Self {
        let topology = CachedTopology::from(&material_map);
        let rgba_field = CachedRgbaField::new(material_map.bounding_rect());
        Self {
            material_map,
            topology,
            rgba_field,
        }
    }

    pub fn topology(&self) -> &Topology {
        self.topology.get_or_init(&self.material_map)
    }

    pub fn material_map(&self) -> &MaterialMap {
        &self.material_map
    }

    pub fn bounding_rect(&self) -> Rect<i64> {
        self.material_map.bounding_rect()
    }

    /// Invalidates all regions keys
    /// Returns true if any regions were changed
    /// Very naive implementation, checks if any Region actually changes material and if
    /// yes recreates the whole topology.
    #[inline(never)]
    pub fn fill_regions(&mut self, fill_regions: &Vec<FillRegion>) -> bool {
        let topology = self
            .topology
            .get_mut()
            .expect("Requires topology, otherwise region ids will be invalid.");

        let mut topology_invalidated = false;
        let mut modified = false;
        for &fill_region in fill_regions {
            let region = &topology[fill_region.region_key];
            if region.material == fill_region.material {
                // already has desired color, skip
                continue;
            }

            modified = true;
            topology.fill_region(
                fill_region.region_key,
                fill_region.material,
                &mut self.material_map,
            );

            self.rgba_field.expire_rgba_rect(region.bounds());

            if topology_invalidated {
                continue;
            }

            if !topology.try_set_region_material(fill_region.region_key, fill_region.material) {
                println!("Topology invalidated!");
                topology_invalidated = true;
            }
        }

        if topology_invalidated {
            self.topology = CachedTopology::empty();
        }
        modified
    }

    pub fn fill_region(&mut self, region_key: RegionKey, material: Material) -> bool {
        let fill_regions = vec![FillRegion {
            region_key,
            material,
        }];
        self.fill_regions(&fill_regions)
    }

    /// Blit passed Pixmap to self.material_map but only where material_map is already defined.
    pub fn blit(&mut self, other: &MaterialMap) {
        self.material_map.blit(other);
        self.topology = CachedTopology::empty();
        self.rgba_field.expire_rgba_rect(other.bounding_rect());
    }

    pub fn rect_selection(&mut self, rect: Rect<i64>) -> Selection {
        let rect = rect.intersect(self.material_map.bounding_rect());
        let mut selection = MaterialMap::nones(rect);
        for pixel in rect.iter_half_open() {
            selection.put(pixel, self.material_map.set(pixel, Material::TRANSPARENT));
        }

        self.rgba_field.expire_rgba_rect(rect);
        self.topology = CachedTopology::empty();
        Selection::new(selection)
    }

    pub fn region_selection(&mut self, region_key: RegionKey) -> Selection {
        let topology = self.topology.get_or_init(&self.material_map);

        let bounds = topology.regions[&region_key].bounds();
        let mut selection = MaterialMap::nones(bounds);
        for pixel in topology.iter_region_interior(region_key) {
            selection.put(pixel, self.material_map.set(pixel, Material::TRANSPARENT));
        }

        self.topology = CachedTopology::empty();
        Selection::new(selection)
    }

    /// Recomputes stale areas of rgba_field, so can be expensive
    pub fn fresh_rgba_field(&self) -> Arc<RwLock<RgbaField>> {
        self.rgba_field.fresh_rgba_field(&self.material_map)
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
