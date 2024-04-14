use crate::{
    math::{pixel::Pixel, rgba8::Rgba8},
    morphism::Morphism,
    pattern::{NullTrace, Search},
    pixmap::PixmapRgba,
    topology::{FillRegion, Topology},
};
use itertools::Itertools;
use std::collections::BTreeSet;

pub struct Rule {
    pub pattern: Topology,
    pub fill_regions: Vec<FillRegion>,
}

impl Rule {
    pub fn new(before: Topology, after: Topology) -> anyhow::Result<Self> {
        let mut fill_regions: Vec<FillRegion> = Vec::new();

        let after_pixmap = after.to_pixmap();

        for (&region_key, region) in &before.regions {
            let Some(fill_color) = Self::fill_color(&after_pixmap, &region.interior.pixels) else {
                anyhow::bail!("Region {region_key} color not constant.")
            };

            if fill_color != region.interior.color {
                let fill_region = FillRegion {
                    region_key,
                    color: fill_color,
                };
                fill_regions.push(fill_region);
            }
        }

        Ok(Rule {
            pattern: before,
            fill_regions,
        })
    }

    /// The fill color of a given region or None if the color is not constant on the region
    /// pixels.
    pub fn fill_color(pixmap: &PixmapRgba, pixels: &BTreeSet<Pixel>) -> Option<Rgba8> {
        pixels
            .iter()
            .map(|&pixel| pixmap[pixel])
            .all_equal_value()
            .ok()
    }

    /// Returns true if there were any changes to the world
    pub fn apply_ops(&self, phi: &Morphism, world: &mut Topology) -> bool {
        let phi_fill_regions = self
            .fill_regions
            .iter()
            .map(|fill_region| FillRegion {
                region_key: phi[fill_region.region_key],
                color: fill_region.color,
            })
            .collect();

        let modified = world.fill_regions(&phi_fill_regions);
        modified
    }
}

pub fn stabilize(world: &mut Topology, rules: &Vec<Rule>) -> usize {
    let mut steps: usize = 0;
    loop {
        let mut applied = false;
        for rule in rules {
            if let Some(phi) = Search::new(world, &rule.pattern).find_first_match(NullTrace::new())
            {
                rule.apply_ops(&phi, world);
                steps += 1;
                applied = true;
            }
        }

        if !applied {
            return steps;
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        bitmap::Bitmap,
        math::rgba8::Rgba8,
        pattern::{NullTrace, Search},
        pixmap::PixmapRgba,
        rule::Rule,
        topology::Topology,
    };

    fn assert_rule_application(folder: &str, expected_application_count: usize) {
        let folder = format!("test_resources/rules/{folder}");
        // let before_pixmap = Pixmap::from_bitmap_path(format!("{folder}/before.png"))
        //     .unwrap()
        //     .without_void_color();
        // let after_pixmap = Pixmap::from_bitmap_path(format!("{folder}/after.png"))
        //     .unwrap()
        //     .without_void_color();

        let before = Topology::from_bitmap_path(format!("{folder}/before.png"))
            .unwrap()
            .without_color(Rgba8::VOID);
        let after = Topology::from_bitmap_path(format!("{folder}/after.png"))
            .unwrap()
            .without_color(Rgba8::VOID);

        let rule = Rule::new(before, after).unwrap();

        let world_bitmap = Bitmap::from_path(format!("{folder}/world.png")).unwrap();
        let mut world = Topology::from_bitmap(&world_bitmap);

        let mut application_count: usize = 0;
        while let Some(phi) = Search::new(&world, &rule.pattern).find_first_match(NullTrace::new())
        {
            rule.apply_ops(&phi, &mut world);
            application_count += 1;
        }

        assert_eq!(application_count, expected_application_count);

        let result_pixmap = world.to_pixmap();
        let expected_result_pixmap =
            PixmapRgba::from_bitmap_path(format!("{folder}/expected_result.png")).unwrap();

        // result_pixmap
        //     .to_bitmap_with_size(world_bitmap.size())
        //     .save(format!("{folder}/world_result.png"))
        //     .unwrap();

        assert_eq!(result_pixmap, expected_result_pixmap);
    }

    #[test]
    fn rule_a() {
        assert_rule_application("a", 4)
    }

    #[test]
    fn rule_b() {
        assert_rule_application("b", 3)
    }

    #[test]
    fn rule_c() {
        assert_rule_application("c", 3)
    }

    #[test]
    fn rule_d() {
        assert_rule_application("d", 3)
    }

    #[test]
    fn rule_circles_1() {
        assert_rule_application("circles_1", 2)
    }

    #[test]
    fn rule_circles_2() {
        assert_rule_application("circles_2", 3)
    }

    #[test]
    fn rule_gate_a() {
        assert_rule_application("gate_a", 2)
    }
}
