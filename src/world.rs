use std::cell::OnceCell;

use crate::{
    material::Material,
    math::rect::Rect,
    pixmap::MaterialMap,
    topology::{FillRegion, RegionKey, Topology},
    view::Selection,
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

    pub fn from(material_map: MaterialMap) -> Self {
        let topology = Topology::new(material_map);
        Self {
            topology: OnceCell::from(topology),
        }
    }

    pub fn get_or_init(&self, material_map: &MaterialMap) -> &Topology {
        self.topology
            .get_or_init(|| Topology::new(material_map.clone()))
    }

    pub fn get(&self) -> Option<&Topology> {
        self.topology.get()
    }

    pub fn get_mut(&mut self) -> Option<&mut Topology> {
        self.topology.get_mut()
    }
}

#[derive(Debug, Clone)]
pub struct World {
    material_map: MaterialMap,

    /// None means the topology has to be recomputed from the material_map
    topology: CachedTopology,
}

impl World {
    pub fn from_material_map(material_map: MaterialMap) -> Self {
        let topology = CachedTopology::from(material_map.clone());
        Self {
            material_map,
            topology,
        }
    }

    pub fn topology(&self) -> &Topology {
        self.topology.get_or_init(&self.material_map)
    }

    pub fn material_map(&self) -> &MaterialMap {
        &self.material_map
    }

    pub fn mut_material_map(&mut self, f: impl FnOnce(&mut MaterialMap)) {
        f(&mut self.material_map);
        self.topology = CachedTopology::empty();
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
            if topology[fill_region.region_key].material == fill_region.material {
                // already has desired color, skip
                continue;
            }

            modified = true;
            topology.fill_region(
                fill_region.region_key,
                fill_region.material,
                &mut self.material_map,
            );

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
    }

    pub fn rect_selection(&mut self, rect: Rect<i64>) -> Selection {
        let rect = rect.intersect(self.material_map.bounding_rect());
        let mut selection = MaterialMap::nones(rect);
        for pixel in rect.iter_half_open() {
            selection.put(pixel, self.material_map.set(pixel, Material::TRANSPARENT));
        }

        self.topology = CachedTopology::empty();
        Selection::new(selection)
    }

    pub fn region_selection(&mut self, region_key: RegionKey) -> Selection {
        let topology = self.topology.get_or_init(&self.material_map);

        let bounds = topology.regions[&region_key].bounds;
        let mut selection = MaterialMap::nones(bounds);
        for pixel in topology.iter_region_interior(region_key) {
            selection.put(pixel, self.material_map.set(pixel, Material::TRANSPARENT));
        }

        self.topology = CachedTopology::empty();
        Selection::new(selection)
    }
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
