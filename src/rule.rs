use crate::{
    bitmap::Bitmap,
    math::{pixel::Pixel, rgba8::Rgba8},
    morphism::Morphism,
    pixmap::Pixmap,
    topology::{RegionKey, Topology},
};
use itertools::Itertools;
use std::{collections::BTreeSet, path::Path};

pub struct FillRegion {
    /// Region key in the pattern, the matched region is filled with `color`
    pub region_key: RegionKey,
    pub color: Rgba8,
}

impl FillRegion {
    pub fn apply(&self, phi: &Morphism, world: &mut Topology) {
        let phi_region_key = phi[&self.region_key];
        world.fill_region(&phi_region_key, self.color);
    }
}

pub enum Op {
    FillRegion(FillRegion),
}

pub struct Rule {
    pattern: Topology,
    ops: Vec<Op>,
}

impl Rule {
    pub fn new(before: Pixmap, after: Pixmap) -> anyhow::Result<Self> {
        let pattern = Topology::new(&before.without_void_color());

        let mut ops: Vec<Op> = Vec::new();

        for (&region_key, region) in &pattern.regions {
            let Some(fill_color) = Self::fill_color(&after, &region.interior) else {
                anyhow::bail!("Region {region_key} color not constant.")
            };

            if fill_color != region.color {
                let op = FillRegion {
                    region_key,
                    color: fill_color,
                };
                ops.push(Op::FillRegion(op));
            }
        }

        Ok(Rule { pattern, ops })
    }

    pub fn from_bitmaps(before: &Bitmap, after: &Bitmap) -> anyhow::Result<Self> {
        let before_pixmap = Pixmap::from_bitmap(before);
        let after_pixmap = Pixmap::from_bitmap(after);
        Self::new(before_pixmap, after_pixmap)
    }

    pub fn from_bitmap_paths(
        before_path: impl AsRef<Path>,
        after_path: impl AsRef<Path>,
    ) -> anyhow::Result<Self> {
        let before = Pixmap::from_bitmap_path(before_path)?;
        let after = Pixmap::from_bitmap_path(after_path)?;
        Self::new(before, after)
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
        for op in &self.ops {
            if let Op::FillRegion(fill_region) = op {
                fill_region.apply(phi, world)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        bitmap::Bitmap,
        pattern::{find_first_match, NullTrace},
        rule::Rule,
        topology::Topology,
    };

    #[test]
    fn rule_a() {
        let folder = "test_resources/rules";
        let rule = Rule::from_bitmap_paths(
            format!("{folder}/a/before.png"),
            format!("{folder}/a/after.png"),
        )
        .unwrap();

        let world_bitmap = Bitmap::from_path(format!("{folder}/a/world.png")).unwrap();
        let mut world = Topology::from_bitmap(&world_bitmap);

        while let Some(phi) = find_first_match(&world, &rule.pattern, NullTrace::new()) {
            println!("Applying rule");
            rule.apply_ops(&phi, &mut world);
        }

        let result_bitmap = world.to_pixmap().to_bitmap_with_size(world_bitmap.size());
        result_bitmap.save(format!("{folder}/a/world_result.png"));
        // Load before, after bitmaps
        // Load world
        // Find first match and apply op
    }
}
