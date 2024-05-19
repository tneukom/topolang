use itertools::Itertools;

use crate::{
    material::Material,
    math::pixel::Pixel,
    morphism::Morphism,
    pattern::{NullTrace, Search},
    pixmap::Pixmap,
    topology::{FillRegion, Topology},
    world::World,
};

pub struct Rule {
    pub pattern: Topology<Material>,
    pub fill_regions: Vec<FillRegion<Material>>,
}

impl Rule {
    pub fn new(before: Topology<Material>, after: Topology<Material>) -> anyhow::Result<Self> {
        let mut fill_regions: Vec<FillRegion<Material>> = Vec::new();

        let after_pixmap = after.material_map();

        for (&before_region_key, before_region) in &before.regions {
            let Some(after_fill_color) = Self::fill_material(
                &after_pixmap,
                before.iter_region_interior(before_region_key),
            ) else {
                anyhow::bail!("Region {before_region_key} color not constant.")
            };

            if after_fill_color != before_region.material {
                let fill_region = FillRegion {
                    region_key: before_region_key,
                    material: after_fill_color,
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
    pub fn fill_material(
        pixmap: &Pixmap<Material>,
        pixels: impl IntoIterator<Item = Pixel>,
    ) -> Option<Material> {
        pixels
            .into_iter()
            .map(|pixel| pixmap[pixel])
            .all_equal_value()
            .ok()
    }

    /// Returns true if there were any changes to the world
    pub fn apply_ops(&self, phi: &Morphism, world: &mut World<Material>) -> bool {
        let phi_fill_regions = self
            .fill_regions
            .iter()
            .map(|fill_region| FillRegion {
                region_key: phi[fill_region.region_key],
                material: fill_region.material,
            })
            .collect();

        let modified = world.fill_regions(&phi_fill_regions);
        modified
    }
}

pub fn stabilize(world: &mut World<Material>, rules: &Vec<Rule>) -> usize {
    let mut steps: usize = 0;
    loop {
        let mut applied = false;
        for rule in rules {
            if let Some(phi) =
                Search::new(world.topology(), &rule.pattern).find_first_match(NullTrace::new())
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
        material::Material,
        pattern::{NullTrace, Search},
        pixmap::MaterialMap,
        rule::Rule,
        topology::Topology,
        world::World,
    };

    fn assert_rule_application(folder: &str, expected_application_count: usize) {
        let folder = format!("test_resources/rules/{folder}");

        let before_material_map = MaterialMap::load(format!("{folder}/before.png"))
            .unwrap()
            .without(&Material::VOID);
        let before = Topology::new(&before_material_map);

        let after_material_map = MaterialMap::load(format!("{folder}/after.png"))
            .unwrap()
            .without(&Material::VOID);
        let after = Topology::new(&after_material_map);

        let rule = Rule::new(before, after).unwrap();

        let world_material_map = MaterialMap::load(format!("{folder}/world.png")).unwrap();
        let mut world = World::<Material>::from_pixmap(world_material_map);

        let mut application_count: usize = 0;
        while let Some(phi) =
            Search::new(world.topology(), &rule.pattern).find_first_match(NullTrace::new())
        {
            rule.apply_ops(&phi, &mut world);
            application_count += 1;
        }

        assert_eq!(application_count, expected_application_count);

        let result_pixmap = world.material_map();
        let expected_result_pixmap =
            MaterialMap::load(format!("{folder}/expected_result.png")).unwrap();

        // result_pixmap
        //     .to_bitmap_with_size(world_bitmap.size())
        //     .save(format!("{folder}/world_result.png"))
        //     .unwrap();

        assert_eq!(result_pixmap, &expected_result_pixmap);
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
