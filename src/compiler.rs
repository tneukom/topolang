use crate::{
    field::RgbaField,
    material::{Material, MaterialClass},
    math::{
        interval::Interval,
        pixel::{Pixel, Side},
        point::Point,
        rect::Rect,
        rgba8::{Rgb8, Rgba8},
    },
    morphism::Morphism,
    new_regions::cancel_opposing_sides,
    pixmap::MaterialMap,
    regions::area_left_of_boundary,
    rule::{InputCondition, InputEvent, Pattern, Rule},
    solver::plan::{
        ConstraintSystem, GuessChooser, GuessChooserUsingStatistics, SearchPlan, SearchStrategy,
        SimpleGuessChooser,
    },
    topology::{
        AtomicTime, Border, BorderKey, MaskedTopology, Region, RegionKey, Topology,
        TopologyStatistics,
    },
    world::World,
};
use ahash::{HashMap, HashSet};
use itertools::Itertools;

#[derive(Debug, Clone)]
pub struct RuleInstance {
    pub before: MaterialMap,

    pub after: MaterialMap,

    pub rule: Rule,
}

/// All regions in the Rule frame, before and after. These elements should be hidden during Rule
/// execution. Contains an arbitrary pixel of each region.
#[derive(Debug, Clone)]
pub struct RuleSource {
    pub keys: HashSet<RegionKey>,
    pub before_outer_border: Border,
    pub after_outer_border: Border,
    pub bounds: Rect<i64>,
    pub modified_time: AtomicTime,
}

impl RuleSource {
    /// Before and after border merged
    pub fn outline(&self) -> HashSet<Side> {
        let before_sides = self.before_outer_border.iter_sides();
        let after_sides = self.after_outer_border.iter_sides();
        let sides = before_sides.chain(after_sides);
        cancel_opposing_sides(sides)
    }

    pub fn area(&self) -> Vec<Pixel> {
        let mut area = area_left_of_boundary(self.before_outer_border.iter_sides());
        area.extend(area_left_of_boundary(self.after_outer_border.iter_sides()));
        area
    }
}

#[derive(Debug, Clone)]
pub struct GenericRule {
    /// Can be None if there is no source.
    pub source: Option<RuleSource>,

    /// Multiple instances if there are placeholders
    pub instances: Vec<RuleInstance>,
}

/// `symbol` and each choice must have same bounds
#[derive(Debug, Clone)]
pub struct PlaceholderRange {
    pub source: HashSet<RegionKey>,

    /// Translated to origin
    pub placeholder: MaterialMap,

    /// Each translated to origin, not empty
    pub choices: Vec<MaterialMap>,
}

impl PlaceholderRange {
    pub fn new(
        placeholder: MaterialMap,
        items: Vec<MaterialMap>,
        source: HashSet<RegionKey>,
    ) -> Self {
        assert!(!items.is_empty());

        let bounds = placeholder.bounding_rect();
        assert_eq!(bounds.low(), Point::ZERO);

        for item in &items {
            assert_eq!(bounds, item.bounding_rect());
        }

        Self {
            placeholder,
            choices: items,
            source,
        }
    }
}

pub struct PlaceholderSubstitution<'a> {
    /// Placeholder to be replaced
    pub placeholder: MaterialMap,

    /// Substitutions to replace placeholder with
    pub range: &'a PlaceholderRange,
}

#[derive(Debug, Clone)]
pub struct CompileError {
    pub bounds: Vec<Rect<i64>>,
    pub message: String,
}

impl CompileError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            bounds: Vec::new(),
        }
    }

    pub fn err<T>(message: impl Into<String>) -> Result<T, Self> {
        Err(Self::new(message))
    }

    pub fn with_bounds(mut self, bounds: Rect<i64>) -> Self {
        self.bounds.push(bounds);
        self
    }
}

// pub trait CompileErrorContext {
//     fn with_bounds(self, bounds: Rect<i64>) -> Self;
// }
//
// impl<T> CompileErrorContext for Result<T, CompileError> {
//     fn with_bounds(self, bounds: Rect<i64>) -> Self {
//         self.map_err(|err| err.with_bounds(bounds))
//     }
// }

#[derive(Debug, Clone)]
pub struct Program {
    pub rules: Vec<GenericRule>,
    pub source: HashSet<RegionKey>,
    pub placeholder_ranges: Vec<PlaceholderRange>,
}

impl Program {
    pub fn iter_rule_instances(
        &self,
    ) -> impl Iterator<Item = (&GenericRule, &RuleInstance)> + Clone {
        self.rules.iter().flat_map(|rule| {
            rule.instances
                .iter()
                .map(move |rule_instance| (rule, rule_instance))
        })
    }

    pub fn rule_instances_len(&self) -> usize {
        self.rules.iter().map(|rule| rule.instances.len()).sum()
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

        let border_region = topology
            .unique_region_by_material(Self::BORDER_MATERIAL)
            .unwrap();

        // Unique outer border
        assert_eq!(border_region.boundary.outer_border().seams_len(), 1);
        let outer_border_key = BorderKey::new(border_region.key(), 0);

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

    before_outer_border_key: BorderKey,

    after_outer_border_key: BorderKey,

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

        let before_frame = rule_frame
            .unique_region_by_material(Material::RULE_BEFORE)
            .unwrap();
        assert_eq!(before_frame.boundary.borders.len(), 1);

        let after_frame = rule_frame
            .unique_region_by_material(Material::RULE_AFTER)
            .unwrap();
        assert_eq!(after_frame.boundary.borders.len(), 1);

        Self {
            before_outer_border_key: BorderKey::new(before_frame.key(), 0),
            after_outer_border_key: BorderKey::new(after_frame.key(), 0),
            input_event_symbols,
            rule_frame,
        }
    }

    pub fn compile_pattern(
        &self,
        mut material_map: MaterialMap,
        guess_chooser: &impl GuessChooser,
    ) -> Result<Pattern, CompileError> {
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

    fn sort_by_interval_key(
        maps: &mut [MaterialMap],
        key: impl Fn(&MaterialMap) -> Interval<i64>,
    ) -> bool {
        maps.sort_by_key(|map| key(map).low);
        let has_no_overlap = maps
            .iter()
            .tuple_windows()
            .all(|(lhs, rhs)| key(lhs).high < key(rhs).low);
        has_no_overlap
    }

    fn sort_horizontally_or_vertically(maps: &mut [MaterialMap]) -> Result<(), CompileError> {
        // && short circuits
        if !Self::sort_by_interval_key(maps, |map| map.bounding_rect().x)
            && !Self::sort_by_interval_key(maps, |map| map.bounding_rect().y)
        {
            return Err(CompileError::new(
                "Failed to sort placeholder choices horizontally or vertically",
            ));
        }
        Ok(())
    }

    #[inline(never)]
    pub fn compile_placeholder_range(
        material_map: &MaterialMap,
        topology: &Topology,
        frame_region: &Region,
    ) -> Result<PlaceholderRange, CompileError> {
        let mut placeholder = None;
        let mut items = Vec::new();

        for hole_border in frame_region.boundary.holes() {
            let inside = material_map.right_of_border(hole_border);

            let is_placeholder = hole_border
                .atomic_seams()
                .all(|seam| topology.material_right_of(seam) == Some(Material::RULE_PLACEHOLDER));
            if is_placeholder {
                if !placeholder.is_none() {
                    return CompileError::err("Only one placeholder allowed");
                }
                placeholder = Some(inside.translated_to_zero());
            } else {
                items.push(inside);
            }
        }

        let Some(placeholder) = placeholder else {
            return CompileError::err("Must have a placeholder");
        };

        Self::sort_horizontally_or_vertically(&mut items)?;

        // Translate all items to zero
        items = items
            .into_iter()
            .map(|item| item.translated_to_zero())
            .collect();

        let source = topology
            .regions_left_of_border(frame_region.boundary.outer_border())
            .map(Region::key)
            .collect();

        let placeholder_range = PlaceholderRange::new(placeholder, items, source);

        Ok(placeholder_range)
    }

    #[inline(never)]
    pub fn compile_placeholder_ranges(
        material_map: &MaterialMap,
        topology: &Topology,
    ) -> Result<Vec<PlaceholderRange>, CompileError> {
        let mut placeholder_ranges = Vec::new();

        for placeholder_frame_candidate in topology.regions.values() {
            if placeholder_frame_candidate.material != Material::RULE_CHOICE {
                continue;
            }

            let placeholder_range = Self::compile_placeholder_range(
                material_map,
                topology,
                placeholder_frame_candidate,
            )
            .map_err(|err| err.with_bounds(placeholder_frame_candidate.bounds()))?;
            placeholder_ranges.push(placeholder_range);
        }

        Ok(placeholder_ranges)
    }

    fn placeholders(material_map: &MaterialMap, topology: &Topology) -> Vec<MaterialMap> {
        topology
            .regions
            .values()
            .filter(|region| region.material == Material::RULE_PLACEHOLDER)
            .map(|region| material_map.left_of_border(region.boundary.outer_border()))
            .collect()
    }

    /// Find range for the given placeholder
    fn placeholder_range<'a>(
        placeholder: &MaterialMap,
        ranges: &'a [PlaceholderRange],
    ) -> Option<&'a PlaceholderRange> {
        // TODO: Avoid clone
        let placeholder = placeholder
            .clone()
            .translated(-placeholder.bounding_rect().low());
        ranges.iter().find(|range| range.placeholder == placeholder)
    }

    fn placeholder_substitutions<'a>(
        material_map: &MaterialMap,
        ranges: &'a [PlaceholderRange],
    ) -> Result<Vec<PlaceholderSubstitution<'a>>, CompileError> {
        let topology = Topology::new(material_map);
        let placeholders = Self::placeholders(material_map, &topology);

        let mut substitutions = Vec::new();
        // Find range for each placeholder
        for placeholder in placeholders {
            let Some(range) = Self::placeholder_range(&placeholder, ranges) else {
                return Err(
                    CompileError::new("No range defined for the given placeholder")
                        .with_bounds(placeholder.bounding_rect()),
                );
            };
            let substitution = PlaceholderSubstitution { placeholder, range };
            substitutions.push(substitution);
        }

        Ok(substitutions)
    }

    fn substitute_placeholder(
        material_map: &mut MaterialMap,
        substitution: &PlaceholderSubstitution,
        i_choice: usize,
    ) {
        material_map.blit_translated(
            &substitution.range.choices[i_choice],
            substitution.placeholder.bounding_rect().low(),
        );
    }

    pub fn compile_rule_instances(
        &self,
        before_material_map: MaterialMap,
        after_material_map: MaterialMap,
        guess_chooser: &impl GuessChooser,
        placeholder_ranges: &Vec<PlaceholderRange>,
    ) -> Result<Vec<RuleInstance>, CompileError> {
        // Find placeholders in before and after, currently we only handle a single placeholder
        let before_substitutions =
            Self::placeholder_substitutions(&before_material_map, placeholder_ranges)?;
        let after_substitutions =
            Self::placeholder_substitutions(&after_material_map, placeholder_ranges)?;

        // Make sure all substitutions have the same length
        let mut substitution_lens = before_substitutions
            .iter()
            .chain(&after_substitutions)
            .map(|substitution| substitution.range.choices.len());

        // If there are no substitutions we set the common_len to one so we can avoid a special
        // case.
        let common_len = substitution_lens.next().unwrap_or(1);

        if substitution_lens.any(|len| len != common_len) {
            return CompileError::err("All substitutions must have the same len");
        }

        let mut rule_instances = Vec::new();
        for i_choice in 0..common_len {
            // TODO: Substitute each before and after placeholder
            let mut before_material_map = before_material_map.clone();
            let mut after_material_map = after_material_map.clone();

            for before_substitution in &before_substitutions {
                Self::substitute_placeholder(
                    &mut before_material_map,
                    before_substitution,
                    i_choice,
                );
            }

            for after_substitution in &after_substitutions {
                Self::substitute_placeholder(&mut after_material_map, after_substitution, i_choice);
            }

            let pattern = self.compile_pattern(before_material_map.clone(), guess_chooser)?;
            let rule = Rule::new(pattern, after_material_map.clone())?;
            let rule_instance = RuleInstance {
                rule,
                before: before_material_map,
                after: after_material_map,
            };
            rule_instances.push(rule_instance);
        }

        Ok(rule_instances)
    }

    pub fn compile_source(&self, topology: &Topology, phi: &Morphism) -> RuleSource {
        // Extract everything inside the before and after regions of the rule
        let phi_before_outer_border = topology[phi[self.before_outer_border_key]].clone();
        let phi_after_outer_border = topology[phi[self.after_outer_border_key]].clone();

        // Collect regions that are part of the source for this Rule.
        let mut keys = HashSet::default();
        // Before and after regions
        let before_keys = topology
            .regions_left_of_border(&phi_before_outer_border)
            .map(Region::key);
        keys.extend(before_keys);
        let after_keys = topology
            .regions_right_of_border(&phi_after_outer_border)
            .map(Region::key);
        keys.extend(after_keys);

        let modified_time = keys
            .iter()
            .map(|&key| topology[key].modified_time)
            .max()
            .unwrap();

        let bounds = Rect::iter_bounds(keys.iter().map(|&key| topology[key].bounds()));

        RuleSource {
            keys,
            before_outer_border: phi_before_outer_border,
            after_outer_border: phi_after_outer_border,
            bounds,
            modified_time,
        }
    }

    #[inline(never)]
    pub fn compile_rules(
        &self,
        material_map: &MaterialMap,
        topology: &Topology,
        placeholder_ranges: &Vec<PlaceholderRange>,
    ) -> Result<Vec<GenericRule>, CompileError> {
        let _tracy_span = tracy_client::span!("compile_rules");

        let masked_topology = MaskedTopology::whole(topology);

        // Find all matches for rule_frame in world
        let matches = {
            let guess_chooser = SimpleGuessChooser::default();
            let constraint_system = ConstraintSystem::for_morphism(&self.rule_frame);
            let search_plan =
                SearchPlan::new(constraint_system, &self.rule_frame, &guess_chooser, None);
            search_plan.solutions(&masked_topology)
        };

        // Extract all matches and creates rules from them
        let mut rules: Vec<GenericRule> = Vec::new();
        // TODO: Statistics should be calculated after hiding found rules!
        let world_statistics = TopologyStatistics::new(&masked_topology);
        let guess_chooser = GuessChooserUsingStatistics::new(world_statistics);

        // Compile each found rule
        for phi in matches {
            let source = self.compile_source(topology, &phi);

            // The before material map is the before frame and everything it contains except
            // RULE_BEFORE
            let before_material_map = material_map
                .left_of_border(&source.before_outer_border)
                .filter(|_, material| material != Material::RULE_BEFORE)
                .shrink();

            let after_material_map = material_map
                .left_of_border(&source.after_outer_border)
                .filter(|_, material| material != Material::RULE_AFTER)
                .shrink();

            // Find translation from after to before
            let before_bounds = before_material_map.bounding_rect();
            let after_bounds = after_material_map.bounding_rect();
            if before_bounds.size() != after_bounds.size() {
                return Err(
                    CompileError::new("Bounds of before and after must be equal.")
                        .with_bounds(source.bounds),
                );
            }

            let offset = before_bounds.low() - after_bounds.low();
            let after_material_map = after_material_map.translated(offset);

            let rule_instances = self
                .compile_rule_instances(
                    before_material_map,
                    after_material_map,
                    &guess_chooser,
                    &placeholder_ranges,
                )
                .map_err(|err| err.with_bounds(source.bounds))?;

            let generic_rule = GenericRule {
                instances: rule_instances,
                source: Some(source),
            };

            rules.push(generic_rule);
        }

        // Sort rules by y coordinates of bounding box
        rules.sort_by_key(|rule| rule.source.as_ref().unwrap().bounds.top());

        Ok(rules)
    }

    pub fn compile(&self, world: &World) -> Result<Program, CompileError> {
        let topology = world.topology();
        let material_map = world.material_map();

        // Compile choice lists
        let placeholder_ranges = Self::compile_placeholder_ranges(material_map, topology)?;

        let rules = self.compile_rules(material_map, topology, &placeholder_ranges)?;

        let mut source = HashSet::default();
        for rule in &rules {
            if let Some(rule_source) = &rule.source {
                source.extend(rule_source.keys.iter().copied())
            }
        }

        for placeholder_range in &placeholder_ranges {
            source.extend(placeholder_range.source.iter().copied())
        }

        Ok(Program {
            rules,
            source,
            placeholder_ranges,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{
        compiler::Compiler, material::Material, math::rgba8::Rgb, pixmap::MaterialMap,
        rule::InputEvent, solver::plan::SimpleGuessChooser, world::World,
    };
    use ahash::HashMap;

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
                .unique_region_by_material(Material::normal(color))
                .unwrap()
                .key()
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

    fn assert_rule_placeholder_substitution(name: &str) {
        let folder = format!("test_resources/compiler/generic/substitution/{name}");
        let compiler = Compiler::new();

        let world = World::load(format!("{folder}/rule.png")).unwrap();
        let program = compiler.compile(&world).unwrap();

        // Save all instances
        for (i, (_, rule_instance)) in program.iter_rule_instances().enumerate() {
            let expected_before = MaterialMap::load(format!("{folder}/before_{i}.png")).unwrap();
            assert_eq!(
                expected_before,
                rule_instance.before.clone().translated_to_zero()
            );

            let expected_after = MaterialMap::load(format!("{folder}/after_{i}.png")).unwrap();
            assert_eq!(
                expected_after,
                rule_instance.after.clone().translated_to_zero()
            );

            // For debugging: save
            // rule_instance
            //     .before
            //     .save())
            //     .unwrap();
            // rule_instance
            //     .after
            //     .save(format!("{folder}/after_{i}.png"))
            //     .unwrap();
        }
    }

    #[test]
    fn rule_placeholder_substitution_before_only() {
        assert_rule_placeholder_substitution("before_only");
    }

    #[test]
    fn rule_placeholder_substitution_before_and_after() {
        assert_rule_placeholder_substitution("before_and_after");
    }
}
