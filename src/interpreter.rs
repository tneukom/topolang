use crate::{
    field::RgbaField,
    material::Material,
    math::{
        pixel::Pixel,
        point::Point,
        rect::{Rect, RectBounds},
        rgba8::Rgba8,
    },
    pixmap::MaterialMap,
    rule::{InputCondition, InputEvent, Pattern, Rule},
    solver::plan::{GuessChooser, GuessChooserUsingStatistics, SearchPlan, SimpleGuessChooser},
    topology::{BorderKey, FillRegion, Region, RegionKey, StrongRegionKey, Topology},
    world::World,
};
use ahash::HashMap;
use std::collections::BTreeSet;

pub struct Ticked {
    pub n_applications: usize,
    pub stabilized: bool,
    pub n_woken_up: usize,
}

impl Ticked {
    /// A rule was applied or a region was woken up.
    pub fn changed(&self) -> bool {
        self.n_applications > 0 || self.n_woken_up > 0
    }
}

pub struct CompiledRule {
    /// All regions in the Rule frame, before and after. These elements should be hidden during Rule
    /// execution. Contains an arbitrary pixel of each region.
    pub source: BTreeSet<StrongRegionKey>,

    pub source_bounds: Rect<i64>,

    pub rule: Rule,
}

pub struct CompiledRules {
    pub rules: Vec<CompiledRule>,
    pub source_region_keys: BTreeSet<StrongRegionKey>,
}

impl CompiledRules {
    /// Returns if a Rule was applied
    #[inline(never)]
    pub fn apply(&self, world: &mut World) -> bool {
        for CompiledRule { rule, .. } in &self.rules {
            let solutions = rule
                .before
                .search_plan
                .solutions_excluding(world.topology(), &self.source_region_keys);

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

    /// Apply rules (at most max_applications) and if world becomes stable under rules wake up
    /// all sleeping regions.
    #[inline(never)]
    pub fn tick(&self, world: &mut World, max_applications: usize) -> Ticked {
        let n_applications = self.stabilize(world, max_applications);
        if n_applications == max_applications {
            return Ticked {
                n_applications,
                stabilized: false,
                n_woken_up: 0,
            };
        }

        let n_woken_up = self.wake_up(world);
        Ticked {
            n_applications,
            stabilized: true,
            n_woken_up,
        }
    }

    /// Apply rules until world is stable or max_applications is reached. Returns the number of
    /// times a rule was applied.
    /// This means if returned value is smaller than max_applications world is stable, otherwise we
    /// don't know if it is stable.
    #[inline(never)]
    pub fn stabilize(&self, world: &mut World, max_applications: usize) -> usize {
        for i in 0..max_applications {
            let applied = self.apply(world);
            if !applied {
                return i;
            }
        }
        max_applications
    }

    /// Wake up all sleeping regions (replace them
    pub fn wake_up(&self, world: &mut World) -> usize {
        let topology = world.topology();

        let mut fill_regions = Vec::new();
        for (&region_key, region) in &topology.regions {
            if region.material.is_sleeping() {
                let strong_region_key = region.top_left_interior_pixel();
                if self.source_region_keys.contains(&strong_region_key) {
                    continue;
                }

                let fill_region = FillRegion {
                    region_key,
                    material: region.material.as_normal(),
                };
                fill_regions.push(fill_region);
            }
        }

        world.fill_regions(&fill_regions);
        fill_regions.len()
    }
}

/// ┌────────────────┐
/// │Wildcard padding│
/// │   ┌────────┐   │
/// │   │Symbol  │   │
/// │   │        │   │
/// │   └────────┘   │
/// └────────────────┘
pub struct Symbol {
    pattern: Pattern,
    padding_region_key: RegionKey,
    padding_inner_border_key: BorderKey,
}

impl Symbol {
    pub fn load(png_bytes: &[u8]) -> Self {
        let material_field = RgbaField::load_from_memory(png_bytes)
            .unwrap()
            .into_material();
        let material_map = MaterialMap::from(material_field);

        // Compile material map into pattern
        let topology = Topology::new(&material_map);
        let guess_chooser = SimpleGuessChooser::default();
        let search_plan = SearchPlan::for_morphism(&topology, &guess_chooser);

        let padding_region_key = topology.region_key_at(Point(0, 0)).unwrap();
        let padding_region = &topology[padding_region_key];

        // Outer border and one inner border
        assert_eq!(padding_region.boundary.borders.len(), 2);
        let padding_inner_border_key = BorderKey::new(padding_region_key, 1);

        let pattern = Pattern {
            material_map,
            topology,
            search_plan,
            input_conditions: Vec::new(),
        };

        Self {
            pattern,
            padding_region_key,
            padding_inner_border_key,
        }
    }

    /// Extract
    pub fn extract_tagged(
        &self,
        topology: &Topology,
        material_map: &mut MaterialMap,
    ) -> Vec<RegionKey> {
        let mut tagged = Vec::new();

        for phi in self.pattern.search_plan.solutions(topology) {
            // Remove right side of padding_inner_border, the actual symbol
            let phi_padding_region_key = phi[self.padding_region_key];
            let phi_padding_inner_border_key = phi[self.padding_inner_border_key];

            let padding_material = topology[phi_padding_region_key].material;
            let padding_inner_border = &topology[phi_padding_inner_border_key];
            material_map.fill_right_of_border(padding_inner_border, padding_material);

            tagged.push(phi_padding_region_key);
        }

        tagged
    }
}

pub struct Compiler {
    rule_frame: Topology,
    before_border: BorderKey,
    after_border: BorderKey,
    input_event_symbols: HashMap<InputEvent, Symbol>,
}

impl Compiler {
    // Not the same as the actual RULE_FRAME color
    const RULE_FRAME_MATERIAL: Material = Material::normal(Rgba8::CYAN.rgb());

    pub fn new() -> Self {
        // Load rule_frame pattern from file
        let rule_frame_rgba_field =
            RgbaField::load_from_memory(include_bytes!("rule_images/rule_frame.png")).unwrap();
        let rule_frame_material_map =
            MaterialMap::from(rule_frame_rgba_field).without(Self::RULE_FRAME_MATERIAL);
        let rule_frame = Topology::new(&rule_frame_material_map);

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

        let input_event_symbols: HashMap<_, _> = InputEvent::ALL
            .into_iter()
            .map(|event| (event, Symbol::load(event.symbol_png())))
            .collect();

        Self {
            rule_frame,
            before_border,
            after_border,
            input_event_symbols,
        }
    }

    pub fn compile_pattern(
        &self,
        mut material_map: MaterialMap,
        guess_chooser: &impl GuessChooser,
    ) -> anyhow::Result<Pattern> {
        let topology = Topology::new(&material_map);

        // Extract symbols from pattern and add InputCondition for each tagged area.
        let mut extracted_symbols = Vec::new();
        for (&event, symbol) in &self.input_event_symbols {
            for tagged_region_key in symbol.extract_tagged(&topology, &mut material_map) {
                let tagged_strong_region_key = topology[tagged_region_key].strong_key();
                extracted_symbols.push((event, tagged_strong_region_key));
            }
        }

        // Update `before` Topology after extracting symbols
        let topology = Topology::new(&material_map);

        let input_conditions = extracted_symbols
            .into_iter()
            .map(|(event, strong_region_key)| InputCondition {
                event,
                region_key: topology.region_key_at(strong_region_key).unwrap(),
            })
            .collect();

        let search_plan = SearchPlan::for_morphism(&topology, guess_chooser);
        Ok(Pattern {
            material_map,
            topology,
            search_plan,
            input_conditions,
        })
    }

    #[inline(never)]
    pub fn compile(&self, world: &World) -> anyhow::Result<CompiledRules> {
        // Find all matches for rule_frame in world
        let matches = {
            let guess_chooser = SimpleGuessChooser::default();
            let search_plan = SearchPlan::for_morphism(&self.rule_frame, &guess_chooser);
            search_plan.solutions(world.topology())
        };

        let topology = world.topology();
        let material_map = world.material_map();

        // Extract all matches and creates rules from them
        let mut rules: Vec<CompiledRule> = Vec::new();
        let world_statistics = world.topology().statistics();
        let guess_chooser = GuessChooserUsingStatistics::new(world_statistics);

        // Compile each found rule
        for phi in matches {
            // Extract before and after from rule
            let phi_before_border = &topology[phi[self.before_border]];
            let before_material_map = material_map.right_of_border(phi_before_border).shrink();
            let before = Topology::new(&before_material_map);

            let phi_after_border = &topology[phi[self.after_border]];
            let after_material_map = material_map.right_of_border(phi_after_border).shrink();
            let after = Topology::new(&after_material_map);

            // Collect regions that are part of the source for this Rule.
            let mut source: BTreeSet<StrongRegionKey> = BTreeSet::new();
            // Before and after regions
            source.extend(before.regions.values().map(Region::strong_key));
            source.extend(after.regions.values().map(Region::strong_key));
            // Rule frame regions
            for &region_key in self.rule_frame.iter_region_keys() {
                let phi_region_key = phi[region_key];
                source.insert(topology[phi_region_key].strong_key());
            }

            let source_bounds = Rect::iter_bounds(
                source
                    .iter()
                    .map(|&key| topology.region_at(key).unwrap().bounds()),
            );

            // Remove rule areas from before and after material maps.
            let before_material_map = before_material_map
                .filter(|_, material| !material.is_rule())
                .shrink();

            let after_material_map = after_material_map
                .filter(|_, material| !material.is_rule())
                .shrink();

            // Find translation from after to before
            let before_bounds = before_material_map.bounding_rect();
            let after_bounds = after_material_map.bounding_rect();
            assert_eq!(before_bounds.size(), after_bounds.size());

            let offset = before_bounds.low() - after_bounds.low();
            let after_material_map = after_material_map.translated(offset);
            let after = Topology::new(&after_material_map);

            let pattern = self.compile_pattern(before_material_map, &guess_chooser)?;

            let rule = Rule::new(pattern, after)?;
            let compiled_rule = CompiledRule {
                rule,
                source,
                source_bounds,
            };
            rules.push(compiled_rule);
        }

        // Sort rules by y coordinates of bounding box
        rules.sort_by_key(|rule| rule.source_bounds.top());

        let source_region_keys: BTreeSet<_> = rules
            .iter()
            .flat_map(|compiled_rule| compiled_rule.source.iter())
            .copied()
            .collect();

        Ok(CompiledRules {
            rules,
            source_region_keys,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{
        field::RgbaField,
        interpreter::Compiler,
        material::Material,
        math::rgba8::Rgb,
        pixmap::MaterialMap,
        rule::InputEvent,
        solver::plan::SimpleGuessChooser,
        utils::{IntoT, KeyValueItertools},
        world::World,
    };
    use ahash::HashMap;
    use itertools::Itertools;

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
            let applied = rules.apply(&mut world);
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

    /// Extract symbols from regions.
    #[test]
    fn tagged_regions() {
        let folder = format!("test_resources/compiler/symbols");
        let compiler = Compiler::new();

        let material_map = MaterialMap::load(format!("{folder}/a.png")).unwrap();
        let guess_chooser = SimpleGuessChooser::default();
        let pattern = compiler
            .compile_pattern(material_map, &guess_chooser)
            .unwrap();

        let expected = MaterialMap::load(format!("{folder}/a_result.png")).unwrap();
        assert_eq!(pattern.material_map, expected);

        // pattern
        //     .material_map
        //     .save(format!("{folder}/a_result.png"))
        //     .unwrap();

        let event_to_region: HashMap<_, _> = pattern
            .input_conditions
            .iter()
            .map(|cond| (cond.event, cond.region_key))
            .collect();

        let region_by_color = |color| {
            pattern
                .topology
                .regions
                .iter()
                .filter_key_by_value(|region| region.material == Material::normal(color))
                .copied()
                .exactly_one()
                .ok()
                .unwrap()
        };

        let expected_event_to_region: HashMap<_, _> = [
            (
                InputEvent::MouseLeftDown,
                region_by_color(Rgb(0x36, 0x90, 0xEA)),
            ),
            (
                InputEvent::MouseOver,
                region_by_color(Rgb(0x6A, 0x5C, 0xFF)),
            ),
            (
                InputEvent::MouseRightDown,
                region_by_color(Rgb(0x00, 0xFF, 0x00)),
            ),
            (
                InputEvent::MouseLeftClick,
                region_by_color(Rgb(0x6D, 0x48, 0x2F)),
            ),
            (
                InputEvent::MouseRightClick,
                region_by_color(Rgb(0x80, 0x80, 0x00)),
            ),
        ]
        .into_iter()
        .collect();

        assert_eq!(expected_event_to_region, event_to_region);

        // for input_condition in &pattern.input_conditions {
        //     println!("{input_condition:?}");
        // }
    }
}
