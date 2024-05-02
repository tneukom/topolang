use crate::{
    bitmap::Bitmap,
    math::{rect::Rect, rgba8::Rgba8},
    pixmap::PixmapRgba,
    topology::{FillRegion, RegionKey, Topology},
};
use std::{cell::OnceCell, path::Path};

struct CachedTopology {
    topology: OnceCell<Topology>,
}

impl CachedTopology {
    pub fn empty() -> Self {
        Self {
            topology: OnceCell::new(),
        }
    }

    pub fn from(color_map: &PixmapRgba) -> Self {
        let topology = Topology::new(&color_map);
        Self {
            topology: OnceCell::from(topology),
        }
    }

    pub fn get_or_init(&self, color_map: &PixmapRgba) -> &Topology {
        self.topology.get_or_init(|| {
            println!("Recomputing topology!");
            Topology::new(color_map)
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
    color_map: PixmapRgba,

    /// None means the topology has to be recomputed from the color_map
    topology: CachedTopology,
}

impl World {
    pub fn from_pixmap(color_map: PixmapRgba) -> Self {
        let topology = CachedTopology::from(&color_map);
        Self {
            color_map,
            topology,
        }
    }

    pub fn from_bitmap(bitmap: &Bitmap) -> Self {
        let color_map = PixmapRgba::from_bitmap(bitmap);
        Self::from_pixmap(color_map)
    }

    pub fn from_bitmap_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let bitmap = Bitmap::from_path(path)?;
        Ok(Self::from_bitmap(&bitmap))
    }

    pub fn topology(&self) -> &Topology {
        self.topology.get_or_init(&self.color_map)
    }

    pub fn color_map(&self) -> &PixmapRgba {
        &self.color_map
    }

    pub fn bounding_rect(&self) -> Rect<i64> {
        self.color_map.bounding_rect()
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
    /// Very naive implementation, checks if any Region actually changes color and if
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
            if topology[fill_region.region_key].color == fill_region.color {
                // already has desired color, skip
                continue;
            }

            modified = true;
            topology.fill_region(
                fill_region.region_key,
                fill_region.color,
                &mut self.color_map,
            );

            if topology_invalidated {
                continue;
            }

            // Try to update the topology to the changed color map
            let can_update_color = topology[fill_region.region_key]
                .iter_seams()
                .all(|seam| topology.color_right_of(seam) != Some(fill_region.color));

            if can_update_color {
                // If all neighboring regions have a different color than fill_region.color we can
                // update the topology by
                let mut_region = &mut topology[fill_region.region_key];
                mut_region.color = fill_region.color;
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

    pub fn fill_region(&mut self, region_key: RegionKey, color: Rgba8) -> bool {
        let fill_regions = vec![FillRegion { region_key, color }];
        self.fill_regions(&fill_regions)
    }

    /// Blit passed Pixmap to self.color_map but only where color_map is already defined.
    pub fn blit_over(&mut self, other: &PixmapRgba) {
        self.color_map.blit_over(other);
        self.topology = CachedTopology::empty();
    }
}

#[cfg(test)]
mod test {
    use crate::{math::rgba8::Rgba8, pixmap::PixmapRgba, world::World};

    fn assert_blit(name: &str) {
        let folder = format!("test_resources/topology/blit/{name}");

        let world_pixmap = PixmapRgba::from_bitmap_path(format!("{folder}/base.png")).unwrap();
        let blit = PixmapRgba::from_bitmap_path(format!("{folder}/blit.png"))
            .unwrap()
            .without_color(&Rgba8::TRANSPARENT);

        let mut expected_pixmap = world_pixmap.clone();
        expected_pixmap.blit_over(&blit);
        let expected_world = World::from_pixmap(expected_pixmap);

        let mut world = World::from_pixmap(world_pixmap);
        world.blit_over(&blit);

        assert_eq!(world.color_map(), expected_world.color_map());
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
