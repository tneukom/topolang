use crate::{
    compiler::Program,
    rule::{CanvasInput, FillRegion, Rule, RuleApplicationContext},
    topology::{AtomicTime, RegionKey},
    utils::monotonic_time,
    world::World,
};
use ahash::HashSet;
use itertools::Itertools;

pub struct Interpreter {
    pub program: Program,

    /// Modification time that each rule has been stabilized up to (inclusive bound).
    pub cursors: Vec<AtomicTime>,
}

#[derive(Debug, Clone, Copy)]
pub enum InterpreterError {
    MaxModificationReached,
}

#[derive(Debug, Clone, Copy)]
pub struct RuleApplication {
    /// Modification time of the source of the rule
    pub source_mtime: Option<AtomicTime>,

    pub real_time: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StabilizeOutcome {
    Stable,
    MaxApplicationsReached,
}

impl Interpreter {
    pub fn new(program: Program) -> Self {
        let cursors = vec![-1; program.rule_instances_len()];
        Self { program, cursors }
    }

    /// Apply rule at any modified region after `cursor` and move `cursor` forward.
    fn apply_rule_with_cursor(
        world: &mut World,
        rule: &Rule,
        ctx: &RuleApplicationContext,
        cursor: &mut AtomicTime,
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
        max_applications: usize,
    ) -> (StabilizeOutcome, Vec<RuleApplication>) {
        let _span = tracy_client::span!("stabilize");

        let ctx = RuleApplicationContext {
            contained: None,
            excluded: &self.program.source,
            input,
        };

        let mut applications = Vec::new();

        'outer: loop {
            if applications.len() >= max_applications {
                return (StabilizeOutcome::MaxApplicationsReached, applications);
            }

            // Stabilize each rule
            for ((generic_rule, rule_instance), cursor) in
                self.program.iter_rule_instances().zip_eq(&mut self.cursors)
            {
                let rule = &rule_instance.rule;

                let modified = if !rule.before.input_conditions.is_empty() {
                    // Modification tracking does not work when rule has input conditions. A rule
                    // can become active even though the Topology hasn't changed.
                    let tracy_span = tracy_client::span!("apply input rule");
                    tracy_span.emit_text(&rule.before.debug_id_str());

                    rule.apply(world, &ctx)
                } else {
                    let tracy_span = tracy_client::span!("apply cursor rule");
                    tracy_span.emit_text(&rule.before.debug_id_str());

                    Self::apply_rule_with_cursor(world, rule, &ctx, cursor)
                };

                if modified {
                    // Start again
                    let application = RuleApplication {
                        real_time: monotonic_time(),
                        source_mtime: generic_rule
                            .source
                            .as_ref()
                            .map(|source| source.modified_time),
                    };
                    applications.push(application);
                    continue 'outer;
                }
            }

            // No modifications were made, so world is stable under all rules
            return (StabilizeOutcome::Stable, applications);
        }
    }

    pub fn wake_up(&mut self, world: &mut World) -> usize {
        wake_up(world, &self.program.source)
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

        let (stabilize_outcome, applications) = self.stabilize(world, input, max_modifications);

        let n_woken_up = if applications.len() == 0 {
            self.wake_up(world)
        } else {
            0
        };

        Ok(Ticked {
            stabilize_outcome,
            applications,
            n_woken_up,
        })
    }
}

pub struct Ticked {
    pub stabilize_outcome: StabilizeOutcome,
    pub applications: Vec<RuleApplication>,
    pub n_woken_up: usize,
}

impl Ticked {
    /// A rule was applied or a region was woken up.
    pub fn changed(&self) -> bool {
        self.applications.len() > 0 || self.n_woken_up > 0
    }
}

/// Wake up all sleeping regions (replace them with normal material). Returns number of regions
/// that were woken up.
pub fn wake_up(world: &mut World, excluded: &HashSet<RegionKey>) -> usize {
    let topology = world.topology();

    // Collect fill region operations before applying them
    let mut fill_regions = Vec::new();
    for (&region_key, region) in &topology.regions {
        if region.material.is_sleeping() {
            if excluded.contains(&region_key) {
                continue;
            }

            let fill_region = FillRegion::new(region_key, region.material.as_normal());
            fill_regions.push(fill_region);
        }
    }

    let n_woken_up = fill_regions.len();
    world.fill_regions(fill_regions);
    n_woken_up
}

#[cfg(test)]
mod test {
    use crate::{
        compiler::Compiler,
        field::RgbaField,
        interpreter::{Interpreter, StabilizeOutcome},
        rule::CanvasInput,
        world::World,
    };

    fn assert_execute_world(name: &str, expected_applications: usize) {
        let folder = format!("test_resources/compiler/{name}/");
        let mut world = World::load(format!("{folder}/world.png")).unwrap();

        let compiler = Compiler::new();
        let rules = compiler.compile(&mut world).unwrap();
        let mut interpreter = Interpreter::new(rules);

        let (outcome, applications) =
            interpreter.stabilize(&mut world, &CanvasInput::default(), 64);
        assert_eq!(outcome, StabilizeOutcome::Stable);
        assert_eq!(applications.len(), expected_applications);

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

    #[test]
    fn generic_a() {
        assert_execute_world("generic/a", 2);
    }
}
