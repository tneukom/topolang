use crate::{
    field::RgbaField,
    material::{Material, MaterialClass},
    math::{
        interval::Interval,
        point::Point,
        rect::Rect,
        rgba8::{Rgb8, Rgba8},
    },
    pixmap::MaterialMap,
    rule::{InputCondition, InputEvent, Pattern, Rule},
    solver::plan::{
        GuessChooser, GuessChooserUsingStatistics, SearchPlan, SearchStrategy, SimpleGuessChooser,
    },
    topology::{BorderKey, MaskedTopology, Region, RegionKey, Topology, TopologyStatistics},
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

#[derive(Debug, Clone)]
pub struct GenericRule {
    /// All regions in the Rule frame, before and after. These elements should be hidden during Rule
    /// execution. Contains an arbitrary pixel of each region.
    /// Can be empty if there is no source.
    pub source: HashSet<RegionKey>,

    pub source_bounds: Rect<i64>,

    /// Multiple instances if there are placeholders
    pub instances: Vec<RuleInstance>,
}

/// `symbol` and each choice must have same bounds
#[derive(Debug, Clone)]
pub struct PlaceholderRange {
    pub source: HashSet<RegionKey>,

    /// Translated to origin
    pub placeholder: MaterialMap,

    /// Each translated to origin
    pub choices: Vec<MaterialMap>,
}

impl PlaceholderRange {
    pub fn new(
        placeholder: MaterialMap,
        items: Vec<MaterialMap>,
        source: HashSet<RegionKey>,
    ) -> Self {
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
pub struct Program {
    pub rules: Vec<GenericRule>,
    pub source: HashSet<RegionKey>,
    pub placeholder_ranges: Vec<PlaceholderRange>,
}

impl Program {
    pub fn iter_rule_instances(&self) -> impl Iterator<Item = &RuleInstance> + Clone {
        self.rules.iter().flat_map(|rule| &rule.instances)
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

        let (before_frame_key, before_frame) = rule_frame
            .regions_by_material(Material::RULE_BEFORE)
            .exactly_one()
            .ok()
            .unwrap();
        assert_eq!(before_frame.boundary.borders.len(), 1);

        let (after_frame_key, after_frame) = rule_frame
            .regions_by_material(Material::RULE_AFTER)
            .exactly_one()
            .ok()
            .unwrap();
        assert_eq!(after_frame.boundary.borders.len(), 1);

        Self {
            rule_frame,
            before_outer_border_key: BorderKey::new(before_frame_key, 0),
            after_outer_border_key: BorderKey::new(after_frame_key, 0),
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

    fn sort_horizontally_or_vertically(maps: &mut [MaterialMap]) -> anyhow::Result<()> {
        // && short circuits
        if !Self::sort_by_interval_key(maps, |map| map.bounding_rect().x)
            && !Self::sort_by_interval_key(maps, |map| map.bounding_rect().y)
        {
            anyhow::bail!("Failed to sort placeholder choices horizontally or vertically");
        }
        Ok(())
    }

    #[inline(never)]
    pub fn compile_placeholder_range(
        material_map: &MaterialMap,
        topology: &Topology,
        region: &Region,
    ) -> anyhow::Result<PlaceholderRange> {
        let mut placeholder = None;
        let mut items = Vec::new();

        for hole_border in region.boundary.holes() {
            let inside = material_map.right_of_border(hole_border);

            let is_placeholder = hole_border
                .atomic_seams()
                .all(|seam| topology.material_right_of(seam) == Some(Material::RULE_PLACEHOLDER));
            if is_placeholder {
                anyhow::ensure!(placeholder.is_none(), "Only one placeholder allowed");
                placeholder = Some(inside.translated_to_zero());
            } else {
                items.push(inside);
            }
        }

        Self::sort_horizontally_or_vertically(&mut items)?;

        // Translate all items to zero
        items = items
            .into_iter()
            .map(|item| item.translated_to_zero())
            .collect();

        let source = topology
            .regions_left_of_border(region.boundary.outer_border())
            .map(Region::key)
            .collect();

        let placeholder_range = PlaceholderRange::new(
            placeholder.ok_or(anyhow::anyhow!("Must have a placeholder"))?,
            items,
            source,
        );

        Ok(placeholder_range)
    }

    #[inline(never)]
    pub fn compile_placeholder_ranges(
        material_map: &MaterialMap,
        topology: &Topology,
    ) -> anyhow::Result<Vec<PlaceholderRange>> {
        let mut placeholder_ranges = Vec::new();

        for region in topology.regions.values() {
            if region.material != Material::RULE_CHOICE {
                continue;
            }

            let placeholder_range =
                Self::compile_placeholder_range(material_map, topology, region)?;
            placeholder_ranges.push(placeholder_range)
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
        for range in ranges {
            println!(
                "range placeholder bounds: {:?}",
                range.placeholder.bounding_rect()
            );
        }
        println!("placeholder bounds: {:?}", placeholder.bounding_rect());

        // TODO: Avoid clone
        let placeholder = placeholder
            .clone()
            .translated(-placeholder.bounding_rect().low());
        ranges.iter().find(|range| range.placeholder == placeholder)
    }

    fn placeholder_substitutions<'a>(
        material_map: &MaterialMap,
        ranges: &'a [PlaceholderRange],
    ) -> anyhow::Result<Vec<PlaceholderSubstitution<'a>>> {
        let topology = Topology::new(material_map);
        let placeholders = Self::placeholders(material_map, &topology);

        let mut substitutions = Vec::new();
        // Find range for each placeholder
        for placeholder in placeholders {
            let Some(range) = Self::placeholder_range(&placeholder, ranges) else {
                anyhow::bail!("No range defined for the given placeholder");
            };
            let substitution = PlaceholderSubstitution { placeholder, range };
            substitutions.push(substitution);
        }

        Ok(substitutions)
    }

    fn substitute_placeholder(
        mut material_map: MaterialMap,
        placeholder: &MaterialMap,
        choice: &MaterialMap,
    ) -> MaterialMap {
        material_map.blit_translated(choice, placeholder.bounding_rect().low());
        material_map
    }

    pub fn compile_rule_instances(
        &self,
        before_material_map: MaterialMap,
        after_material_map: MaterialMap,
        guess_chooser: &impl GuessChooser,
        placeholder_ranges: &Vec<PlaceholderRange>,
    ) -> anyhow::Result<Vec<RuleInstance>> {
        // Find placeholders in before and after, currently we only handle a single placeholder
        let before_substitutions =
            Self::placeholder_substitutions(&before_material_map, placeholder_ranges)?;
        let after_substitutions =
            Self::placeholder_substitutions(&after_material_map, placeholder_ranges)?;

        let Ok(before_substitution) = before_substitutions.into_iter().at_most_one() else {
            anyhow::bail!("More than one before placeholder not supported yet.")
        };

        let Ok(after_substitution) = after_substitutions.into_iter().at_most_one() else {
            anyhow::bail!("More than one after placeholder not supported yet.")
        };

        let before_after_instances: Vec<_> = if let Some(before_substitution) = &before_substitution
            && let Some(after_substitution) = &after_substitution
        {
            anyhow::ensure!(
                before_substitution.range.choices.len() == after_substitution.range.choices.len(),
                "range of before and after choices must be same length"
            );

            before_substitution
                .range
                .choices
                .iter()
                .zip_eq(&after_substitution.range.choices)
                .map(|(before_choice, after_choice)| {
                    // Substitute before and after into placeholders
                    let before_material_map = Self::substitute_placeholder(
                        before_material_map.clone(),
                        &before_substitution.placeholder,
                        before_choice,
                    );

                    let after_material_map = Self::substitute_placeholder(
                        after_material_map.clone(),
                        &after_substitution.placeholder,
                        after_choice,
                    );

                    (before_material_map, after_material_map)
                })
                .collect()
        } else if let Some(before_substitution) = &before_substitution {
            before_substitution
                .range
                .choices
                .iter()
                .map(|before_choice| {
                    // Substitute before into placeholders
                    let before_material_map = Self::substitute_placeholder(
                        before_material_map.clone(),
                        &before_substitution.placeholder,
                        before_choice,
                    );

                    (before_material_map, after_material_map.clone())
                })
                .collect()
        } else if let Some(after_substitution) = &after_substitution {
            anyhow::bail!("Placeholder only in the after side does not make sense");
        } else {
            vec![(before_material_map, after_material_map)]
        };

        let mut rule_instances = Vec::new();
        for (before_material_map, after_material_map) in before_after_instances {
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

    #[inline(never)]
    pub fn compile_rules(
        &self,
        material_map: &MaterialMap,
        topology: &Topology,
        placeholder_ranges: &Vec<PlaceholderRange>,
    ) -> anyhow::Result<Vec<GenericRule>> {
        let masked_topology = MaskedTopology::whole(topology);

        // Find all matches for rule_frame in world
        let matches = {
            let guess_chooser = SimpleGuessChooser::default();
            let search_plan = SearchPlan::for_morphism(&self.rule_frame, &guess_chooser, None);
            search_plan.solutions(&masked_topology)
        };

        // Extract all matches and creates rules from them
        let mut rules: Vec<GenericRule> = Vec::new();
        // TODO: Statistics should be calculated after hiding found rules!
        let world_statistics = TopologyStatistics::new(&masked_topology);
        let guess_chooser = GuessChooserUsingStatistics::new(world_statistics);

        // Compile each found rule
        for phi in matches {
            // Extract everything inside the before and after regions of the rule
            let phi_before_border = &topology[phi[self.before_outer_border_key]];
            let phi_after_border = &topology[phi[self.after_outer_border_key]];

            // Collect regions that are part of the source for this Rule.
            let mut source = HashSet::default();
            // Before and after regions
            let before_source = topology
                .regions_left_of_border(phi_before_border)
                .map(Region::key);
            source.extend(before_source);
            let after_source = topology
                .regions_right_of_border(phi_after_border)
                .map(Region::key);
            source.extend(after_source);

            let source_bounds = Rect::iter_bounds(source.iter().map(|&key| topology[key].bounds()));

            // The before material map is the before frame and everything it contains except
            // RULE_BEFORE
            let before_material_map = material_map
                .left_of_border(phi_before_border)
                .filter(|_, material| material != Material::RULE_BEFORE)
                .shrink();

            let after_material_map = material_map
                .left_of_border(phi_after_border)
                .filter(|_, material| material != Material::RULE_AFTER)
                .shrink();

            // Find translation from after to before
            let before_bounds = before_material_map.bounding_rect();
            let after_bounds = after_material_map.bounding_rect();
            assert_eq!(before_bounds.size(), after_bounds.size());

            let offset = before_bounds.low() - after_bounds.low();
            let after_material_map = after_material_map.translated(offset);

            let rule_instances = self.compile_rule_instances(
                before_material_map,
                after_material_map,
                &guess_chooser,
                &placeholder_ranges,
            )?;

            let generic_rule = GenericRule {
                instances: rule_instances,
                source,
                source_bounds,
            };

            rules.push(generic_rule);
        }

        // Sort rules by y coordinates of bounding box
        rules.sort_by_key(|rule| rule.source_bounds.top());

        Ok(rules)
    }

    pub fn compile(&self, world: &World) -> anyhow::Result<Program> {
        let topology = world.topology();
        let material_map = world.material_map();

        // Compile choice lists
        let placeholder_ranges = Self::compile_placeholder_ranges(material_map, topology)?;

        let rules = self.compile_rules(material_map, topology, &placeholder_ranges)?;

        let mut source = HashSet::default();
        for rule in &rules {
            source.extend(rule.source.iter().copied())
        }

        for placeholder_range in &placeholder_ranges {
            source.extend(placeholder_range.source.iter().copied())
        }

        Ok(Program {
            rules,
            source,
            placeholder_ranges: placeholder_ranges,
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

    fn assert_rule_placeholder_substitution(name: &str) {
        let folder = format!("test_resources/compiler/generic/substitution/{name}");
        let compiler = Compiler::new();

        let world = World::load(format!("{folder}/rule.png")).unwrap();
        let program = compiler.compile(&world).unwrap();

        // Save all instances
        for (i, rule_instance) in program.iter_rule_instances().enumerate() {
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
