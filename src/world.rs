use std::cell::OnceCell;

use crate::{
    material::Material,
    math::rect::Rect,
    pixmap::MaterialMap,
    regions::left_of_boundary,
    rule::FillBoundary,
    topology::{RegionKey, Topology},
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

    pub fn mut_material_map(&mut self, mut f: impl FnMut(&mut MaterialMap)) {
        f(&mut self.material_map);
        self.topology = CachedTopology::empty();
    }

    pub fn bounding_rect(&self) -> Rect<i64> {
        self.material_map.bounding_rect()
    }

    pub fn fill_boundary(&mut self, fill: &FillBoundary) {
        self.topology = CachedTopology::empty();
        for pixel in left_of_boundary(fill.boundary.iter().copied()) {
            // TODO: Setting material map one pixel at a time is very slow
            self.material_map.set(pixel, fill.material);
        }
    }

    pub fn fill_region(&mut self, region_key: RegionKey, material: Material) -> bool {
        let topology = self.topology.get_or_init(&self.material_map);
        if topology[region_key].material == material {
            // already has desired color, skip
            return false;
        }

        topology.fill_region(region_key, material, &mut self.material_map);
        self.topology = CachedTopology::empty();
        true
    }

    /// Blit passed Pixmap to self.material_map but only where material_map is already defined.
    pub fn blit_over(&mut self, other: &MaterialMap) {
        self.material_map.blit_over(other);
        self.topology = CachedTopology::empty();
    }

    pub fn fill_rect(&mut self, rect: Rect<i64>, material: Material) {
        self.material_map.fill_rect(rect, material);
        self.topology = CachedTopology::empty();
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
            .without(&Material::TRANSPARENT);

        let mut expected_pixmap = world_pixmap.clone();
        expected_pixmap.blit_over(&blit);
        let expected_world = World::from_material_map(expected_pixmap);

        let mut world = World::from_material_map(world_pixmap);
        world.blit_over(&blit);

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
