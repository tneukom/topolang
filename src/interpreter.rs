use crate::{
    compiler::CompiledRules,
    rule::{CanvasInput, FillRegion, Rule, RuleApplicationContext},
    topology::{ModificationTime, StrongRegionKey},
    world::World,
};
use itertools::Itertools;
use std::collections::BTreeSet;

pub struct Interpreter {
    pub rules: CompiledRules,

    /// Modification time that each rule has been stabilized up to (inclusive bound).
    pub cursors: Vec<ModificationTime>,
}

#[derive(Debug, Clone, Copy)]
pub enum InterpreterError {
    MaxModificationReached,
}

impl Interpreter {
    pub fn new(rules: CompiledRules) -> Self {
        let cursors = vec![-1; rules.rules.len()];
        Self { rules, cursors }
    }

    /// Apply rule at any modified region after `cursor` and move `cursor` forward.
    fn apply_rule_with_cursor(
        world: &mut World,
        rule: &Rule,
        ctx: &RuleApplicationContext,
        cursor: &mut ModificationTime,
    ) -> bool {
        while let Some((mtime, modified_region_key)) =
            world.topology().first_modification_after(*cursor)
        {
            assert!(mtime > *cursor);

            let apply_ctx = RuleApplicationContext {
                contained: Some(modified_region_key),
                ..*ctx
            };
            let modified = rule.apply(world, &apply_ctx);
            if modified {
                // info!("modified");
                return true;
            } else {
                // info!(
                //     "not modified, advancing cursor from {} to {}",
                //     *cursor, mtime
                // );
                *cursor = mtime;
            }
        }

        false
    }

    pub fn stabilize(
        &mut self,
        world: &mut World,
        input: &CanvasInput,
        max_modifications: usize,
    ) -> Result<usize, InterpreterError> {
        let _span = tracy_client::span!("stabilize");

        let ctx = RuleApplicationContext {
            contained: None,
            excluded: &self.rules.source_region_keys,
            input,
        };

        let mut n_modifications = 0usize;

        'outer: loop {
            if n_modifications >= max_modifications {
                return Err(InterpreterError::MaxModificationReached);
            }

            // Stabilize each rule
            for (rule, cursor) in self.rules.rules.iter().zip_eq(&mut self.cursors) {
                let modified = if !rule.rule.before.input_conditions.is_empty() {
                    // Modification tracking does not work when rule has input conditions. A rule
                    // can become active even though the Topology hasn't changed.
                    let tracy_span = tracy_client::span!("apply input rule");
                    tracy_span.emit_text(&rule.rule.before.debug_id_str());

                    rule.rule.apply(world, &ctx)
                } else {
                    let tracy_span = tracy_client::span!("apply cursor rule");
                    tracy_span.emit_text(&rule.rule.before.debug_id_str());

                    Self::apply_rule_with_cursor(world, &rule.rule, &ctx, cursor)
                };

                if modified {
                    // Start again
                    n_modifications += 1;
                    continue 'outer;
                }
            }

            // No modifications were made, so world is stable under all rules
            return Ok(n_modifications);
        }
    }

    pub fn wake_up(&mut self, world: &mut World) -> usize {
        wake_up(world, &self.rules.source_region_keys)
    }

    /// Apply rules (at most max_applications) and if world becomes stable under rules wake up
    /// all sleeping regions.
    #[inline(never)]
    pub fn tick(
        &mut self,
        world: &mut World,
        input: &CanvasInput,
        max_modifications: usize,
    ) -> Result<Ticked, InterpreterError> {
        let _span = tracy_client::span!("tick");

        let n_modifications = self.stabilize(world, input, max_modifications)?;
        let n_woken_up = self.wake_up(world);

        Ok(Ticked {
            n_modifications,
            n_woken_up,
        })
    }
}

pub struct Ticked {
    pub n_modifications: usize,
    pub n_woken_up: usize,
}

impl Ticked {
    /// A rule was applied or a region was woken up.
    pub fn changed(&self) -> bool {
        self.n_modifications > 0 || self.n_woken_up > 0
    }
}

/// Wake up all sleeping regions (replace them with normal material). Returns number of regions
/// that were woken up.
pub fn wake_up(world: &mut World, excluded: &BTreeSet<StrongRegionKey>) -> usize {
    let topology = world.topology();

    // Collect fill region operations before applying them
    let mut fill_regions = Vec::new();
    for (&region_key, region) in &topology.regions {
        if region.material.is_sleeping() {
            let strong_region_key = region.top_left_interior_pixel();
            if excluded.contains(&strong_region_key) {
                continue;
            }

            let fill_region = FillRegion {
                region_key,
                material: region.material.as_normal(),
            };
            fill_regions.push(fill_region);
        }
    }

    for fill_region in &fill_regions {
        world.fill_region(fill_region.region_key, fill_region.material);
    }
    fill_regions.len()
}

#[cfg(test)]
mod test {
    use crate::{
        compiler::Compiler, field::RgbaField, interpreter::Interpreter, pixmap::MaterialMap,
        rule::CanvasInput, world::World,
    };

    fn assert_execute_world(name: &str, expected_modifications: usize) {
        let folder = format!("test_resources/compiler/{name}/");
        let material_map = MaterialMap::load(format!("{folder}/world.png")).unwrap();
        let mut world = World::from_material_map(material_map);

        let compiler = Compiler::new();
        let rules = compiler.compile(&mut world).unwrap();
        let mut interpreter = Interpreter::new(rules);

        let n_modifications = interpreter
            .stabilize(&mut world, &CanvasInput::default(), 64)
            .unwrap();
        assert_eq!(n_modifications, expected_modifications);

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

    /// Failure case where execution fails when using Topology::update but not when rebuilding
    /// using Topology::new.
    #[test]
    fn fail_topology_update() {
        assert_execute_world("fail_topology_update", 1);
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
