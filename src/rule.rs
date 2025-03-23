use crate::{
    morphism::Morphism,
    solver::plan::{GuessChooser, SearchPlan},
    topology::{FillRegion, RegionKey, Topology},
    world::World,
};
use itertools::Itertools;

pub struct Rule {
    /// The pattern
    pub before: Topology,

    /// The substitution
    pub after: Topology,

    /// Plan for finding morphisms
    pub search_plan: SearchPlan,
}

impl Rule {
    pub fn assert_phi_region_constant(
        dom: &Topology,
        codom: &Topology,
        region_key: RegionKey,
    ) -> anyhow::Result<()> {
        // REVISIT: Could be faster by iterating over both before.region_map and
        //   after.region_map in parallel.
        let all_equal = dom
            .iter_region_interior(region_key)
            .map(|pixel| codom.material_at(pixel).unwrap())
            .all_equal();
        if !all_equal {
            anyhow::bail!("Invalid rule, substitution region not constant.")
        }

        Ok(())
    }

    #[inline(never)]
    pub fn new(
        before: Topology,
        after: Topology,
        guess_chooser: &impl GuessChooser,
    ) -> anyhow::Result<Self> {
        // Make sure after region map is constant on each region in `before` and `holes`.
        for &region_key in before.regions.keys() {
            Self::assert_phi_region_constant(&before, &after, region_key)?
        }

        let search_plan = SearchPlan::for_morphism(&before, guess_chooser);

        Ok(Rule {
            before,
            after,
            search_plan,
        })
    }

    /// Given a match for the pattern `self.before` and the world, apply the substitution determined
    /// by `self.before` and `self.after`
    /// Returns true if there were any changes to the world
    pub fn substitute(&self, phi: &Morphism, world: &mut World) -> bool {
        let mut fill_regions = Vec::new();
        for (region_key, before_region) in &self.before.regions {
            let after_material = self
                .after
                .material_at(before_region.top_left_interior_pixel())
                .unwrap();
            if before_region.material == after_material {
                continue;
            }

            let phi_region_key = phi.region_map[region_key];
            let fill_region = FillRegion {
                region_key: phi_region_key,
                material: after_material,
            };

            fill_regions.push(fill_region);
        }

        let modified = world.fill_regions(&fill_regions);
        modified
    }
}

pub fn stabilize(world: &mut World, rules: &Vec<Rule>) -> usize {
    let mut steps: usize = 0;
    loop {
        let mut applied = false;
        for rule in rules {
            if let Some(phi) = rule.search_plan.first_solution(world.topology()) {
                rule.substitute(&phi, world);
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
        field::RgbaField,
        pixmap::MaterialMap,
        rule::Rule,
        solver::plan::{SearchPlan, SimpleGuessChooser},
        topology::Topology,
        world::World,
    };

    fn assert_rule_application(folder: &str, expected_application_count: usize) {
        let folder = format!("test_resources/rules/{folder}");

        let before_material_map = MaterialMap::load(format!("{folder}/before.png"))
            .unwrap()
            .filter(|_, material| !material.is_rule());
        let before = Topology::new(&before_material_map);

        let after_material_map = MaterialMap::load(format!("{folder}/after.png")).unwrap();
        let after = Topology::new(&after_material_map);

        let guess_chooser = SimpleGuessChooser::default();
        let rule = Rule::new(before, after, &guess_chooser).unwrap();

        let world_material_map = MaterialMap::load(format!("{folder}/world.png")).unwrap();
        let mut world = World::from_material_map(world_material_map);

        let mut application_count: usize = 0;

        let guess_chooser = SimpleGuessChooser::default();
        let search_plan = SearchPlan::for_morphism(&rule.before, &guess_chooser);

        while let Some(phi) = search_plan.solutions(world.topology()).first() {
            let changed = rule.substitute(&phi, &mut world);
            if !changed {
                break;
            }

            // Save world to image for debugging!
            // world
            //     .topology()
            //     .material_map
            //     .save(format!("{folder}/run_{application_count}.png"))
            //     .unwrap();

            application_count += 1;
            assert!(application_count <= expected_application_count);
        }

        assert_eq!(application_count, expected_application_count);

        let result_pixmap = world.material_map();
        let expected_result_pixmap = RgbaField::load(format!("{folder}/expected_result.png"))
            .unwrap()
            .into();

        // result_pixmap
        //     .save(format!("{folder}/world_result.png"))
        //     .unwrap();

        assert_eq!(result_pixmap, &expected_result_pixmap);
    }

    #[test]
    fn basic_1() {
        assert_rule_application("basic_1", 1)
    }

    #[test]
    fn rule_a() {
        assert_rule_application("a", 3)
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

    #[test]
    fn rule_interior_hole() {
        assert_rule_application("interior_hole", 1)
    }

    #[test]
    fn rule_two_regions_failure_case() {
        assert_rule_application("two_regions_failure_case", 1)
    }

    #[test]
    fn wildcard_1() {
        assert_rule_application("wildcard_1", 1)
    }

    #[test]
    fn wildcard_2() {
        assert_rule_application("wildcard_2", 1)
    }
}
