use crate::{
    bitmap::Bitmap,
    math::{pixel::Pixel, rgba8::Rgba8},
    morphism::Morphism,
    pattern::{find_first_match, NullTrace},
    pixmap::Pixmap,
    topology::{FillRegion, Topology},
};
use itertools::Itertools;
use std::{collections::BTreeSet, path::Path};

pub struct Rule {
    pub pattern: Topology,
    pub fill_regions: Vec<FillRegion>,
}

impl Rule {
    pub fn new(before: &Pixmap, after: &Pixmap) -> anyhow::Result<Self> {
        let pattern = Topology::new(before);

        let mut fill_regions: Vec<FillRegion> = Vec::new();

        for (&region_key, region) in &pattern.regions {
            let Some(fill_color) = Self::fill_color(after, &region.interior) else {
                anyhow::bail!("Region {region_key} color not constant.")
            };

            if fill_color != region.color {
                let fill_region = FillRegion {
                    region_key,
                    color: fill_color,
                };
                fill_regions.push(fill_region);
            }
        }

        Ok(Rule {
            pattern,
            fill_regions,
        })
    }

    pub fn from_bitmaps(before: &Bitmap, after: &Bitmap) -> anyhow::Result<Self> {
        let before_pixmap = Pixmap::from_bitmap(before).without_void_color();
        let after_pixmap = Pixmap::from_bitmap(after).without_void_color();
        Self::new(&before_pixmap, &after_pixmap)
    }

    pub fn from_bitmap_paths(
        before_path: impl AsRef<Path>,
        after_path: impl AsRef<Path>,
    ) -> anyhow::Result<Self> {
        let before = Bitmap::from_path(before_path)?;
        let after = Bitmap::from_path(after_path)?;
        Self::from_bitmaps(&before, &after)
    }

    /// The fill color of a given region or None if the color is not constant on the region
    /// pixels.
    pub fn fill_color(pixmap: &Pixmap, pixels: &BTreeSet<Pixel>) -> Option<Rgba8> {
        pixels
            .iter()
            .map(|pixel| pixmap[pixel])
            .all_equal_value()
            .ok()
    }

    pub fn apply_ops(&self, phi: &Morphism, world: &mut Topology) {
        let phi_fill_regions = self
            .fill_regions
            .iter()
            .map(|fill_region| FillRegion {
                region_key: phi[fill_region.region_key],
                color: fill_region.color,
            })
            .collect();

        world.fill_regions(&phi_fill_regions);
    }
}

pub fn stabilize(world: &mut Topology, rules: &Vec<Rule>) -> usize {
    let mut steps: usize = 0;
    loop {
        let mut applied = false;
        for rule in rules {
            if let Some(phi) = find_first_match(world, &rule.pattern, NullTrace::new()) {
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
        pattern::{find_first_match, NullTrace},
        pixmap::Pixmap,
        rule::Rule,
        topology::Topology,
    };

    fn assert_rule_application(folder: &str, expected_application_count: usize) {
        let folder = format!("test_resources/rules/{folder}");
        let rule = Rule::from_bitmap_paths(
            format!("{folder}/before.png"),
            format!("{folder}/after.png"),
        )
        .unwrap();

        let world_bitmap = Bitmap::from_path(format!("{folder}/world.png")).unwrap();
        let mut world = Topology::from_bitmap(&world_bitmap);

        let mut application_count: usize = 0;
        while let Some(phi) = find_first_match(&world, &rule.pattern, NullTrace::new()) {
            rule.apply_ops(&phi, &mut world);
            application_count += 1;
        }

        assert_eq!(application_count, expected_application_count);

        let result_pixmap = world.to_pixmap();
        let expected_result_pixmap =
            Pixmap::from_bitmap_path(format!("{folder}/expected_result.png")).unwrap();

        assert_eq!(result_pixmap, expected_result_pixmap);

        // result_bitmap.save(format!("{folder}/world_result.png"));
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
    fn rule_gate_a() {
        assert_rule_application("gate_a", 2)
    }
}
