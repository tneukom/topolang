use crate::{
    math::point::Point,
    morphism::Morphism,
    pixmap::MaterialMap,
    solver::plan::SearchStrategy,
    topology::{FillRegion, RegionKey, StrongRegionKey, Topology},
    world::World,
};
use itertools::Itertools;
use std::{collections::BTreeSet, fmt::format};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputEvent {
    MouseLeftDown,
    MouseLeftClick,
    MouseRightDown,
    MouseRightClick,
    MouseOver,
}

impl InputEvent {
    pub fn symbol_png(self) -> &'static [u8] {
        match self {
            Self::MouseLeftDown => include_bytes!("rule_images/mouse_left_down.png"),
            Self::MouseLeftClick => include_bytes!("rule_images/mouse_left_click.png"),
            Self::MouseRightDown => include_bytes!("rule_images/mouse_right_down.png"),
            Self::MouseRightClick => include_bytes!("rule_images/mouse_right_click.png"),
            Self::MouseOver => include_bytes!("rule_images/mouse_over.png"),
        }
    }

    pub const ALL: [Self; 5] = [
        Self::MouseLeftDown,
        Self::MouseLeftClick,
        Self::MouseRightDown,
        Self::MouseRightClick,
        Self::MouseOver,
    ];
}

#[derive(Debug, Clone)]
pub struct InputCondition {
    pub event: InputEvent,
    pub region_key: RegionKey,
}

#[derive(Debug, Clone, Default)]
pub struct CanvasInput {
    pub mouse_position: Point<i64>,
    pub left_mouse_down: bool,
    pub left_mouse_click: bool,
    pub right_mouse_down: bool,
    pub right_mouse_click: bool,
}

impl InputCondition {
    pub fn is_satisfied(&self, phi: &Morphism, codom: &Topology, input: &CanvasInput) -> bool {
        // The given region contains the mouse cursor
        let phi_region_key = phi[self.region_key];
        let contains_mouse = codom.region_key_at(input.mouse_position) == Some(phi_region_key);

        match self.event {
            InputEvent::MouseLeftDown => contains_mouse && input.left_mouse_down,
            InputEvent::MouseLeftClick => contains_mouse && input.left_mouse_click,
            InputEvent::MouseRightDown => contains_mouse && input.right_mouse_down,
            InputEvent::MouseRightClick => contains_mouse && input.right_mouse_click,
            InputEvent::MouseOver => contains_mouse,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub material_map: MaterialMap,

    /// The pattern
    pub topology: Topology,

    pub search_strategy: SearchStrategy,

    pub input_conditions: Vec<InputCondition>,
}

impl Pattern {
    pub fn input_conditions_satisfied(
        &self,
        phi: &Morphism,
        codom: &Topology,
        input: &CanvasInput,
    ) -> bool {
        self.input_conditions
            .iter()
            .all(|cond| cond.is_satisfied(&phi, codom, input))
    }

    pub fn debug_id_str(&self) -> String {
        let before_bounds = self.material_map.bounding_rect();
        format!("({}, {})", before_bounds.left(), before_bounds.top())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RuleApplicationContext<'a> {
    pub contained: Option<RegionKey>,
    pub excluded: &'a BTreeSet<StrongRegionKey>,
    pub input: &'a CanvasInput,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub before: Pattern,

    /// The substitution
    pub after: Topology,
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
    pub fn new(before: Pattern, after: Topology) -> anyhow::Result<Self> {
        // Make sure after region map is constant on each region in `before` and `holes`.
        for &region_key in before.topology.regions.keys() {
            Self::assert_phi_region_constant(&before.topology, &after, region_key)?
        }

        Ok(Rule { before, after })
    }

    /// Given a match for the pattern `self.before` and the world, apply the substitution determined
    /// by `self.before` and `self.after`
    /// Returns true if there were any changes to the world
    pub fn substitute(&self, phi: &Morphism, world: &mut World) -> bool {
        let mut fill_regions = Vec::new();
        for (region_key, before_region) in &self.before.topology.regions {
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

    /// Apply rule until `world` has changed or is stable under `rule`, only matches that contain
    /// `contained` are considered.
    /// Returns true if a modification was made.
    pub fn apply(&self, world: &mut World, ctx: &RuleApplicationContext) -> bool {
        let tracy_span = tracy_client::span!("Rule::apply");
        tracy_span.emit_color(0xFFFFFF);
        // let debug_id_str = ;
        // println!("{}", debug_id_str);
        tracy_span.emit_text(&self.before.debug_id_str());
        // tracy_span.emit_text("wtf???");

        let topology = world.topology();
        let solutions =
            self.before
                .search_strategy
                .solutions(topology, ctx.contained, ctx.excluded);

        for phi in solutions {
            // Check if input conditions are satisfied
            if !self
                .before
                .input_conditions_satisfied(&phi, world.topology(), ctx.input)
            {
                continue;
            }

            let modified = self.substitute(&phi, world);
            if modified {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod test {
    use crate::{
        field::RgbaField,
        pixmap::MaterialMap,
        rule::{Pattern, Rule},
        solver::plan::{SearchStrategy, SimpleGuessChooser},
        topology::Topology,
        world::World,
    };
    use std::collections::BTreeSet;

    fn assert_rule_application(folder: &str, expected_application_count: usize) {
        let folder = format!("test_resources/rules/{folder}");

        let before_material_map = MaterialMap::load(format!("{folder}/before.png"))
            .unwrap()
            .filter(|_, material| !material.is_rule());
        let before = Topology::new(&before_material_map);

        let after_material_map = MaterialMap::load(format!("{folder}/after.png")).unwrap();
        let after = Topology::new(&after_material_map);

        let guess_chooser = SimpleGuessChooser::default();
        let search_strategy = SearchStrategy::for_morphism(&before, &guess_chooser);
        let pattern = Pattern {
            material_map: before_material_map,
            topology: before,
            search_strategy,
            input_conditions: Vec::new(),
        };
        let rule = Rule::new(pattern, after).unwrap();

        let world_material_map = MaterialMap::load(format!("{folder}/world.png")).unwrap();
        let mut world = World::from_material_map(world_material_map);

        let mut application_count: usize = 0;

        while let Some(phi) = rule
            .before
            .search_strategy
            .solutions(world.topology(), None, &BTreeSet::default())
            .first()
        {
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

    #[test]
    fn disjoint() {
        assert_rule_application("disjoint", 1)
    }
}
