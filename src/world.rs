use std::cell::OnceCell;

use crate::{
    material::Material,
    math::rect::Rect,
    pixmap::MaterialMap,
    topology::{FillRegion, RegionKey, Topology},
};

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
        let topology = Topology::new(&material_map);
        Self {
            topology: OnceCell::from(topology),
        }
    }

    pub fn get_or_init(&self, material_map: &MaterialMap) -> &Topology {
        self.topology.get_or_init(|| {
            println!("Recomputing topology!");
            Topology::new(material_map)
        })
    }

    pub fn get(&self) -> Option<&Topology> {
        self.topology.get()
    }

    pub fn get_mut(&mut self) -> Option<&mut Topology> {
        self.topology.get_mut()
    }
}

pub struct World {
    material_map: MaterialMap,

    /// None means the topology has to be recomputed from the material_map
    topology: CachedTopology,
}

impl World {
    pub fn from_pixmap(material_map: MaterialMap) -> Self {
        let topology = CachedTopology::from(&material_map);
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

    pub fn bounding_rect(&self) -> Rect<i64> {
        self.material_map.bounding_rect()
    }

    // Previous implementation to update partial Topology
    // pub fn blit(&mut self, pixmap: &PixmapRgba) {
    //     // Find regions that are touched by `pixels`
    //     let touched_regions = self.touched_regions(pixmap.keys());
    //
    //     // Blit pixels to the Pixmap
    //     self.color_map.blit(pixmap);
    //
    //     // Convert Pixmap back to Topology
    //     let topology = Topology::new(&extracted_pixmap);
    //
    //     // Blit topology region_map back to self
    //     self.region_map.blit(&topology.region_map);
    //
    //     // Merge topology with self
    //     for (region_key, region) in topology.regions.into_iter() {
    //         self.insert_region(region_key, region);
    //     }
    // }

    // #[inline(never)]
    // fn fill_regions_fallback(&mut self, fill_regions: &Vec<FillRegion>) {
    //     let mut color_map = self.color_map();
    //
    //     for fill_region in fill_regions.into_iter() {
    //         for pixel in self.iter_region_interior(fill_region.region_key) {
    //             color_map.set(pixel, fill_region.color);
    //         }
    //     }
    //
    //     *self = Self::new(&color_map)
    // }

    // /// Invalidates all regions keys
    // /// Returns true if any regions were changed
    // #[inline(never)]
    // pub fn fill_regions(&mut self, fill_regions: &Vec<FillRegion>) -> bool {
    //     let mut modified = false;
    //     let mut remaining = Vec::new();
    //
    //     for &fill_region in fill_regions {
    //         // If all neighboring regions have a different color than fill_region.color we can
    //         // simply replace Region.color
    //         let region = &self[fill_region.region_key];
    //
    //         if region.color == fill_region.color {
    //             // already has desired color, skip
    //             continue;
    //         }
    //
    //         modified = true;
    //
    //         if region
    //             .iter_seams()
    //             .all(|seam| self.color_right_of(seam) != Some(fill_region.color))
    //         {
    //             let mut_region = &mut self[fill_region.region_key];
    //             mut_region.color = fill_region.color;
    //         } else {
    //             remaining.push(fill_region)
    //         }
    //     }
    //
    //     if !remaining.is_empty() {
    //         self.fill_regions_fallback(&remaining);
    //     }
    //
    //     modified
    // }

    /// Invalidates all regions keys
    /// Returns true if any regions were changed
    /// Very naive implementation, checks if any Region actually changes material and if
    /// yes recreates the whole topology.
    #[inline(never)]
    pub fn fill_regions(&mut self, fill_regions: &Vec<FillRegion>) -> bool {
        let mut modified = false;
        let mut topology_invalidated = false;
        let topology = self
            .topology
            .get_mut()
            .expect("Requires topology, otherwise region ids will be invalid.");

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

            // Try to update the topology to the changed material map
            let can_update_material = topology[fill_region.region_key]
                .iter_seams()
                .all(|seam| topology.material_right_of(seam) != Some(fill_region.material));

            if can_update_material {
                // If all neighboring regions have a different material than fill_region.material we
                // can update the topology by
                let mut_region = &mut topology[fill_region.region_key];
                mut_region.material = fill_region.material;
            } else {
                // Otherwise we have to recompute the topology
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
    pub fn blit_over(&mut self, other: &MaterialMap) {
        self.material_map.blit_over(other);
        self.topology = CachedTopology::empty();
    }
}

impl From<MaterialMap> for World {
    fn from(material_map: MaterialMap) -> Self {
        Self::from_pixmap(material_map)
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
        let expected_world = World::from_pixmap(expected_pixmap);

        let mut world = World::from_pixmap(world_pixmap);
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
