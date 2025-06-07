use crate::{
    field::RgbaField,
    material::{Material, MaterialClass},
    math::{
        rect::Rect,
        rgba8::{Rgb8, Rgba8},
    },
    pixmap::MaterialMap,
    rule::{InputCondition, InputEvent, Pattern, Rule},
    solver::plan::{
        GuessChooser, GuessChooserUsingStatistics, SearchPlan, SearchStrategy, SimpleGuessChooser,
    },
    topology::{BorderKey, MaskedTopology, RegionKey, Topology, TopologyStatistics},
    world::World,
};
use ahash::{HashMap, HashSet};
use itertools::Itertools;

#[derive(Debug, Clone)]
pub struct CompiledRule {
    /// All regions in the Rule frame, before and after. These elements should be hidden during Rule
    /// execution. Contains an arbitrary pixel of each region.
    pub source: HashSet<RegionKey>,

    pub source_bounds: Rect<i64>,

    pub rule: Rule,
}

#[derive(Debug, Clone)]
pub struct CompiledRules {
    pub rules: Vec<CompiledRule>,
    pub source_region_keys: HashSet<RegionKey>,
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
    outer_border_key: BorderKey,
}

impl Symbol {
    pub const BORDER_MATERIAL: Material = Material::new(Rgb8::MAGENTA, MaterialClass::Solid);

    pub fn load(png_bytes: &[u8]) -> Self {
        let material_field = RgbaField::load_from_memory(png_bytes)
            .unwrap()
            .into_material();
        let material_map = MaterialMap::from(material_field).without(Material::TRANSPARENT);

        // Compile material map into pattern
        let topology = Topology::new(&material_map);
        let guess_chooser = SimpleGuessChooser::default();
        let search_strategy = SearchStrategy::for_morphism(&topology, &guess_chooser);

        let (border_region_key, border_region) = topology
            .regions_by_material(Self::BORDER_MATERIAL)
            .exactly_one()
            .ok()
            .unwrap();

        // Unique outer border
        assert_eq!(border_region.boundary.outer_border().seams_len(), 1);
        let outer_border_key = BorderKey::new(border_region_key, 0);

        let pattern = Pattern {
            material_map,
            topology,
            search_strategy,
            input_conditions: Vec::new(),
        };

        Self {
            pattern,
            outer_border_key,
        }
    }

    /// Extract found symbols from `material_map` and return the regions they were part of. The
    /// holes left from the symbols are filled with the material that surrounds them, meaning there
    /// cannot be different materials around the symbol.
    pub fn extract_symbols(
        &self,
        topology: &Topology,
        material_map: &mut MaterialMap,
    ) -> Vec<RegionKey> {
        let mut tagged = Vec::new();

        let solutions = self
            .pattern
            .search_strategy
            .main_plan
            .solutions(&topology.into());

        for phi in solutions {
            let phi_outer_border_key = phi[self.outer_border_key];
            let phi_outer_border = &topology[phi_outer_border_key];

            // Border must be a single loop seam
            let Ok(phi_outer_seam) = phi_outer_border.atomic_seams().exactly_one() else {
                println!("Symbol cannot be surrounded by multiple different materials.");
                continue;
            };

            let surrounding_region_key = topology.right_of(phi_outer_seam).unwrap();
            let surrounding_region = &topology[surrounding_region_key];

            // Remove left side of phi_outer_border (the symbol) and fill with the surrounding
            // material.
            material_map.fill_left_of_border(phi_outer_border, surrounding_region.material);

            tagged.push(surrounding_region_key);
        }

        tagged
    }
}

pub struct Compiler {
    rule_frame: Topology,
    before_border_key: BorderKey,
    after_border_key: BorderKey,
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

        let input_event_symbols: HashMap<_, _> = InputEvent::ALL
            .into_iter()
            .map(|event| (event, Symbol::load(event.symbol_png())))
            .collect();

        let (before_frame_key, _) = rule_frame
            .regions_by_material(Material::RULE_BEFORE)
            .exactly_one()
            .ok()
            .unwrap();
        let before_border_key = BorderKey::new(before_frame_key, 0);

        let (after_frame_key, _) = rule_frame
            .regions_by_material(Material::RULE_AFTER)
            .exactly_one()
            .ok()
            .unwrap();
        let after_border_key = BorderKey::new(after_frame_key, 0);

        Self {
            rule_frame,
            before_border_key,
            after_border_key,
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
        let mut input_conditions = Vec::new();
        for (&event, symbol) in &self.input_event_symbols {
            for tagged_region_key in symbol.extract_symbols(&topology, &mut material_map) {
                let input_condition = InputCondition {
                    event,
                    region_key: tagged_region_key,
                };
                input_conditions.push(input_condition);
            }
        }

        // Update `before` Topology after extracting symbols
        let topology = Topology::new(&material_map);

        let search_strategy = SearchStrategy::for_morphism(&topology, guess_chooser);

        Ok(Pattern {
            material_map,
            topology,
            search_strategy,
            input_conditions,
        })
    }

    #[inline(never)]
    pub fn compile(&self, world: &World) -> anyhow::Result<CompiledRules> {
        let topology = world.topology();
        let material_map = world.material_map();
        let masked_topology = MaskedTopology::whole(world.topology());

        // Find all matches for rule_frame in world
        let matches = {
            let guess_chooser = SimpleGuessChooser::default();
            let search_plan = SearchPlan::for_morphism(&self.rule_frame, &guess_chooser, None);
            search_plan.solutions(&masked_topology)
        };

        // Extract all matches and creates rules from them
        let mut rules: Vec<CompiledRule> = Vec::new();
        // TODO: Statistics should be calculated after hiding found rules!
        let world_statistics = TopologyStatistics::new(&masked_topology);
        let guess_chooser = GuessChooserUsingStatistics::new(world_statistics);

        // Compile each found rule
        for phi in matches {
            // Extract everything inside the before and after regions of the rule

            let phi_before_border = &topology[phi[self.before_border_key]];
            let before_material_map = material_map
                .left_of_border(phi_before_border)
                .filter(|_, material| !material.is_rule())
                .shrink();
            let before = Topology::new(&before_material_map);

            let phi_after_border = &topology[phi[self.after_border_key]];
            let after_material_map = material_map
                .left_of_border(phi_after_border)
                .filter(|_, material| !material.is_rule())
                .shrink();
            let after = Topology::new(&after_material_map);

            // Collect regions that are part of the source for this Rule.
            let mut source = HashSet::default();
            // Before and after regions
            source.extend(before.regions.keys());
            source.extend(after.regions.keys());
            // Rule frame regions
            for region_key in self.rule_frame.iter_region_keys() {
                let phi_region_key = phi[region_key];
                source.insert(phi_region_key);
            }

            let source_bounds = Rect::iter_bounds(source.iter().map(|&key| topology[key].bounds()));

            // Find translation from after to before
            let before_bounds = before_material_map.bounding_rect();
            let after_bounds = after_material_map.bounding_rect();
            assert_eq!(before_bounds.size(), after_bounds.size());

            let offset = before_bounds.low() - after_bounds.low();
            let after_material_map = after_material_map.translated(offset);
            let after = Topology::new(&after_material_map);

            let pattern = self.compile_pattern(before_material_map, &guess_chooser)?;

            let rule = Rule::new(pattern, after, &after_material_map)?;
            let compiled_rule = CompiledRule {
                rule,
                source,
                source_bounds,
            };
            rules.push(compiled_rule);
        }

        // Sort rules by y coordinates of bounding box
        rules.sort_by_key(|rule| rule.source_bounds.top());

        let source_region_keys: HashSet<_> = rules
            .iter()
            .flat_map(|compiled_rule| &compiled_rule.source)
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
        compiler::Compiler, material::Material, math::rgba8::Rgb, pixmap::MaterialMap,
        rule::InputEvent, solver::plan::SimpleGuessChooser,
    };
    use ahash::HashMap;
    use itertools::Itertools;

    #[test]
    fn init() {
        let _compiler = Compiler::new();
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

        // For debugging
        // pattern
        //     .material_map
        //     .save(format!("{folder}/a_result.png"))
        //     .unwrap();

        let expected = MaterialMap::load(format!("{folder}/a_result.png")).unwrap();
        assert_eq!(pattern.material_map, expected);

        let event_to_region: HashMap<_, _> = pattern
            .input_conditions
            .iter()
            .map(|cond| (cond.event, cond.region_key))
            .collect();

        let region_by_color = |color| {
            pattern
                .topology
                .regions_by_material(Material::normal(color))
                .exactly_one()
                .ok()
                .unwrap()
                .0
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
