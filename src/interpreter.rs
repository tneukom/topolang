use std::collections::BTreeSet;

use crate::solver::plan::SearchPlan;
use crate::{
    field::RgbaField,
    material::Material,
    math::{pixel::Pixel, rgba8::Rgba8},
    pixmap::MaterialMap,
    rule::Rule,
    topology::{BorderKey, Region, StrongRegionKey, Topology},
    utils::IntoT,
    world::World,
};

pub struct CompiledRule {
    /// All regions in the Rule frame, before and after. These elements should be hidden during Rule
    /// execution. Contains an arbitrary pixel of each region.
    source: BTreeSet<StrongRegionKey>,

    rule: Rule,
}

pub struct CompiledRules {
    rules: Vec<CompiledRule>,
}

impl CompiledRules {
    /// Returns if a Rule was applied
    #[inline(never)]
    pub fn step(&self, world: &mut World) -> bool {
        let hidden: BTreeSet<_> = self
            .rules
            .iter()
            .flat_map(|compiled_rule| compiled_rule.source.iter())
            .copied()
            .collect();

        for CompiledRule { rule, source, .. } in &self.rules {
            let solutions = rule.search_plan.solutions_excluding(world.topology(), source);

            // TODO: Reject solutions with hidden elements

            for phi in solutions {
                let modified = rule.substitute(&phi, world);
                if modified {
                    return true;
                }
            }
        }

        false

        // Save hidden regions to bitmap
        // world
        //     .clone()
        //     .sub_topology(hidden)
        //     .to_pixmap()
        //     .to_bitmap_with_size(world_bitmap.size())
        //     .save(format!("{folder}/hidden.png"))
        //     .unwrap();
    }
}

pub struct Compiler {
    rule_frame: Topology,
    before_border: BorderKey,
    after_border: BorderKey,
}

impl Compiler {
    // Not the same as the actual RULE_FRAME color
    const RULE_FRAME_MATERIAL: Material = Material::from_rgba(Rgba8::CYAN);

    pub fn new() -> Self {
        // Load rule_frame pattern from file
        let rule_frame_pixmap = RgbaField::load_from_memory(include_bytes!("rule_frame.png"))
            .unwrap()
            .intot::<MaterialMap>()
            .without(Self::RULE_FRAME_MATERIAL);
        let rule_frame = Topology::new(rule_frame_pixmap);

        // Side on the before border (inner border of the frame)
        let before_side = Pixel::new(7, 7).top_side().reversed();
        let before_border = rule_frame
            .border_containing_side(&before_side)
            .expect("Before border not found.")
            .0;

        let after_side = Pixel::new(43, 8).top_side().reversed();
        let after_border = rule_frame
            .border_containing_side(&after_side)
            .expect("After border not found.")
            .0;

        Self {
            rule_frame,
            before_border,
            after_border,
        }
    }

    #[inline(never)]
    pub fn compile(&self, world: &World) -> anyhow::Result<CompiledRules> {
        // Find all matches for rule_frame in world
        let matches = {
            let search_plan = SearchPlan::for_morphism(&self.rule_frame);
            search_plan.solutions(world.topology())
        };

        let topology = world.topology();

        // Extract all matches and creates rules from them
        let mut compiled_rules: Vec<CompiledRule> = Vec::new();

        for phi in matches {
            // Extract before and after from rule
            let phi_before_border = phi[self.before_border];
            let before = topology.topology_right_of_border(&topology[phi_before_border]);

            let phi_after_border = phi[self.after_border];
            let after = topology.topology_right_of_border(&topology[phi_after_border]);

            // Collect regions that are part of the source for this Rule.
            let mut source: BTreeSet<StrongRegionKey> = BTreeSet::new();
            // Before and after regions
            source.extend(
                before
                    .regions
                    .values()
                    .map(Region::top_left_interior_pixel),
            );
            source.extend(after.regions.values().map(Region::top_left_interior_pixel));
            // Rule frame regions
            for &region_key in self.rule_frame.iter_region_keys() {
                let phi_region_key = phi[region_key];
                source.insert(topology[phi_region_key].top_left_interior_pixel());
            }

            let before = before.filter_by_material(Material::is_not_void);
            let after = after.filter_by_material(Material::is_not_void);

            // Find translation from after to before
            let before_bounds = before.not_none_bounding_rect();
            let after_bounds = after.not_none_bounding_rect();
            assert_eq!(before_bounds.size(), after_bounds.size());

            let after = after.translated(before_bounds.low() - after_bounds.low());

            let rule = Rule::new(before, after)?;
            let compiled_rule = CompiledRule { rule, source };
            compiled_rules.push(compiled_rule);
        }

        Ok(CompiledRules {
            rules: compiled_rules,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{
        field::RgbaField, interpreter::Compiler, pixmap::MaterialMap, utils::IntoT, world::World,
    };

    #[test]
    fn init() {
        let _compiler = Compiler::new();
    }

    fn assert_execute_world(name: &str, expected_steps: usize) {
        let folder = format!("test_resources/compiler/{name}/");
        let mut world = RgbaField::load(format!("{folder}/world.png"))
            .unwrap()
            .intot::<MaterialMap>()
            .intot::<World>();

        let compiler = Compiler::new();
        let rules = compiler.compile(&mut world).unwrap();

        let mut steps = 0;
        loop {
            let applied = rules.step(&mut world);
            if !applied {
                break;
            }
            steps += 1;

            // Save world to image for debugging!
            // world
            //     .material_map()
            //     .to_field(Material::VOID)
            //     .into_rgba()
            //     .save(format!("{folder}/run_{name}_{steps}.png"))
            //     .unwrap();

            assert!(steps <= expected_steps);
        }

        // let steps = stabilize(&mut world, &rules);
        // println!("Number of steps: {steps}");
        assert_eq!(steps, expected_steps);

        let result_pixmap = world.material_map();

        let expected_pixmap = RgbaField::load(format!("{folder}/world_expected.png"))
            .unwrap()
            .into();
        assert_eq!(result_pixmap, &expected_pixmap);
    }

    #[test]
    fn basic_1() {
        assert_execute_world("basic_1", 1);
    }

    #[test]
    fn a() {
        assert_execute_world("a", 3);
    }

    #[test]
    fn b() {
        assert_execute_world("b", 7);
    }

    #[test]
    fn c() {
        assert_execute_world("c", 2);
    }

    /// Failure case: Rule was applied to its own source.
    #[test]
    fn fail_rule_applied_to_its_source() {
        assert_execute_world("fail_rule_applied_to_its_source", 1);
    }

    /// Failure case: Rule was applied to its own source.
    #[test]
    fn fail_noop() {
        assert_execute_world("fail_noop", 0);
    }

    #[test]
    fn gate() {
        assert_execute_world("gate", 3);
    }

    #[test]
    fn hole() {
        assert_execute_world("hole", 2);
    }

    #[test]
    fn circles() {
        assert_execute_world("circles", 3);
    }
}
