use crate::{
    material::Material,
    math::{pixel::Pixel, point::Point},
    morphism::Morphism,
    pixmap::MaterialMap,
    solver::plan::SearchStrategy,
    topology::{MaskedTopology, RegionKey, StrongRegionKey, Topology},
    world::World,
};
use itertools::Itertools;
use std::collections::BTreeSet;

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
pub struct FillRegion {
    /// Region key in the pattern, the matched region is filled with `material`
    pub region_key: RegionKey,
    pub material: Material,
}

/// Draw operation on a matched Region. Assumes the matched Region has the same shape as the pattern
/// Region.
#[derive(Debug, Clone)]
pub struct DrawRegion {
    pub region_key: RegionKey,

    // Pixels are relative to the top-left pixel of the region
    pub pixel_materials: Vec<(Pixel, Material)>,
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

    pub fills: Vec<FillRegion>,

    pub draws: Vec<DrawRegion>,
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
        before: Pattern,
        after: Topology,
        after_material_map: &MaterialMap,
    ) -> anyhow::Result<Self> {
        // Compute fill operations to be applied
        let mut fills = Vec::new();
        let mut draws = Vec::new();
        for (&before_region_key, before_region) in &before.topology.regions {
            if before_region.material.is_solid() {
                let mut pixel_materials = Vec::new();
                for pixel in before_region.boundary.interior_area() {
                    let before_material = before.material_map.get(pixel).unwrap();
                    let after_material = after_material_map.get(pixel).unwrap();
                    if before_material != after_material {
                        pixel_materials.push((pixel, after_material));
                    }
                }

                if pixel_materials.is_empty() {
                    continue;
                }

                let draw = DrawRegion {
                    region_key: before_region_key,
                    pixel_materials,
                };
                draws.push(draw);

                continue;
            }

            // Make sure after region map is constant on each region except solid regions.
            // TODO: Write function instead that returns the constant color over a region or
            //   an error.
            Self::assert_phi_region_constant(&before.topology, &after, before_region_key)?;

            let after_material = after
                .material_at(before_region.top_left_interior_pixel())
                .unwrap();

            if before_region.material == after_material {
                continue;
            }

            let fill_region = FillRegion {
                region_key: before_region_key,
                material: after_material,
            };

            fills.push(fill_region);
        }

        // Compute draw operations to be applied

        Ok(Rule {
            before,
            after,
            fills,
            draws,
        })
    }

    /// Given a match for the pattern `self.before` and the world, apply the substitution determined
    /// by `self.before` and `self.after`
    /// Returns true if there were any changes to the world
    pub fn substitute(&self, phi: &Morphism, world: &mut World) -> bool {
        let mut modified = false;

        for fill_region in &self.fills {
            let phi_region_key = phi[fill_region.region_key];
            modified |= world.fill_region(phi_region_key, fill_region.material);
        }

        for draw_region in &self.draws {
            let phi_region_key = phi[draw_region.region_key];
            // RegionKey is the minimal side of the outer cycle, so left side is the minimal pixel.
            let offset = phi_region_key.left_pixel;
            let offset_material_pixels: Vec<_> = draw_region
                .pixel_materials
                .iter()
                .map(|&(pixel, material)| (pixel + offset, material))
                .collect();
            modified |= world.draw(offset_material_pixels.into_iter());
        }

        modified
    }

    /// Apply rule until `world` has changed or is stable under `rule`, only matches that contain
    /// `contained` are considered.
    /// Returns true if a modification was made.
    pub fn apply(&self, world: &mut World, ctx: &RuleApplicationContext) -> bool {
        let tracy_span = tracy_client::span!("Rule::apply");
        tracy_span.emit_color(0xFFFFFF);
        tracy_span.emit_text(&self.before.debug_id_str());

        let topology = world.topology();
        let masked_topology = MaskedTopology::new(topology, ctx.excluded);
        let solutions = self
            .before
            .search_strategy
            .solutions(&masked_topology, ctx.contained);

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
        let rule = Rule::new(pattern, after, &after_material_map).unwrap();

        let world_material_map = MaterialMap::load(format!("{folder}/world.png")).unwrap();
        let mut world = World::from_material_map(world_material_map);

        let mut application_count: usize = 0;

        while let Some(phi) = rule
            .before
            .search_strategy
            .solutions(&world.topology().into(), None)
            .first()
        {
            let changed = rule.substitute(&phi, &mut world);
            if !changed {
                break;
            }

            // Save world to image for debugging!
            // world
            //     .material_map()
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

    #[test]
    fn solid_draw_1() {
        assert_rule_application("solid_draw_1", 3)
    }

    #[test]
    fn solid_draw_2() {
        assert_rule_application("solid_draw_2", 2)
    }
}
