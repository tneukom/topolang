use crate::{
    bitmap::Bitmap,
    math::{pixel::Pixel, rgba8::Rgba8},
    pattern::{NullTrace, Search},
    pixmap::PixmapRgba,
    rule::Rule,
    topology::{BorderKey, RegionKey, Topology},
};
use std::collections::BTreeSet;

pub struct CompiledRule {
    /// All regions in the Rule frame, before and after. These elements should be hidden during Rule execution.
    source: Vec<RegionKey>,

    rule: Rule,
}

pub struct Interpreter {
    rule_frame: Topology,
    before_border: BorderKey,
    after_border: BorderKey,
}

impl Interpreter {
    const RULE_FRAME_VOID_COLOR: Rgba8 = Rgba8::CYAN;

    pub fn new() -> Self {
        // Load rule_frame pattern from file
        let rule_frame_bitmap = Bitmap::load_from_memory(include_bytes!("rule_frame.png")).unwrap();
        let rule_frame_pixmap =
            PixmapRgba::from_bitmap(&rule_frame_bitmap).without_color(&Self::RULE_FRAME_VOID_COLOR);
        let rule_frame = Topology::new(&rule_frame_pixmap);

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

    pub fn compile(&self, world: &mut Topology) -> anyhow::Result<Vec<CompiledRule>> {
        // Find all matches for rule_frame in world
        let search = Search::new(world, &self.rule_frame);
        let matches = search.find_matches(NullTrace::new());

        // Extract all matches and creates rules from them
        let mut compiled_rules: Vec<CompiledRule> = Vec::new();

        for phi in matches {
            // Extract before and after from rule
            let phi_before_border = phi[self.before_border];
            let before = world.topology_right_of_border(&world[phi_before_border]);

            let phi_after_border = phi[self.after_border];
            let after = world.topology_right_of_border(&world[phi_after_border]);

            // Collect regions that are part of the source for this Rule
            let mut source: Vec<_> = Vec::new();
            source.extend(before.regions.keys().copied());
            source.extend(after.regions.keys().copied());
            for &region_key in self.rule_frame.iter_region_keys() {
                source.push(phi[region_key]);
            }

            let before = before.without_color(Rgba8::VOID);
            let after = after.without_color(Rgba8::VOID);

            // Find translation from after to before
            let before_bounds = before.actual_bounds();
            let after_bounds = after.actual_bounds();
            assert_eq!(before_bounds.size(), after_bounds.size());

            let offset = before_bounds.low() - after_bounds.low();
            let after = after.translated(offset);

            let rule = Rule::new(before, after)?;
            let compiled_rule = CompiledRule { rule, source };
            compiled_rules.push(compiled_rule);
        }

        Ok(compiled_rules)
    }

    /// Returns if a Rule was applied
    pub fn step(&self, world: &mut Topology) -> bool {
        let compiled_rules = self.compile(world).unwrap();

        let hidden: BTreeSet<_> = compiled_rules
            .iter()
            .flat_map(|compiled_rule| compiled_rule.source.iter())
            .copied()
            .collect();

        for CompiledRule { rule, .. } in &compiled_rules {
            let mut search = Search::new(&world, &rule.pattern);
            search.hidden = Some(&hidden);

            let solutions = search.find_matches(NullTrace::new());

            for phi in solutions {
                let modified = rule.apply_ops(&phi, world);
                if modified {
                    return true;
                }
                // world
                //     .to_pixmap()
                //     .to_bitmap_with_size(world_bitmap.size())
                //     .save(format!("{folder}/stabilize_{steps}.png"))
                //     .unwrap();
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

#[cfg(test)]
mod test {
    use crate::{bitmap::Bitmap, interpreter::Interpreter, pixmap::PixmapRgba, topology::Topology};
    use pretty_assertions::assert_eq;

    #[test]
    fn init() {
        let _compiler = Interpreter::new();
    }

    fn assert_execute_world(name: &str, expected_steps: usize) {
        let folder = format!("test_resources/compiler/{name}/");
        let world_bitmap = Bitmap::from_path(format!("{folder}/world.png")).unwrap();
        let world_pixmap = PixmapRgba::from_bitmap(&world_bitmap);
        let mut world = Topology::new(&world_pixmap);

        let compiler = Interpreter::new();

        let mut steps = 0;
        loop {
            let applied = compiler.step(&mut world);
            if !applied {
                break;
            }
            steps += 1;
            assert!(steps <= expected_steps);
        }

        // Apply a rule
        // let mut steps: usize = 0;
        // loop {
        //     let mut applied = false;
        //     for rule in &rules {
        //         if let Some(phi) =
        //             Search::new(&world, &rule.pattern).find_first_match(NullTrace::new())
        //         {
        //             rule.apply_ops(&phi, &mut world);
        //             steps += 1;
        //             applied = true;
        //             world
        //                 .to_pixmap()
        //                 .to_bitmap_with_size(world_bitmap.size())
        //                 .save(format!("{folder}/stabilize_{steps}.png"))
        //                 .unwrap();
        //         }
        //     }
        //
        //     if !applied {
        //         return break;
        //     }
        // }

        // let steps = stabilize(&mut world, &rules);
        println!("Number of steps: {steps}");
        assert_eq!(steps, expected_steps);

        let result_pixmap = world.color_map();

        let expected_pixmap =
            PixmapRgba::from_bitmap_path(format!("{folder}/world_expected.png")).unwrap();
        assert_eq!(result_pixmap, expected_pixmap);

        // let result_bitmap = result_pixmap.to_bitmap_with_size(world_bitmap.size());
        // result_bitmap
        //     .save(format!("{folder}/world_result.png"))
        //     .unwrap();
    }

    #[test]
    fn a() {
        assert_execute_world("a", 4);
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
