use crate::{
    material::Material,
    morphism::Morphism,
    solver::{
        constraints::{AnyConstraint, Constraint, Variables, morphism_constraints},
        element::Element,
        propagations::{AnyPropagation, Propagation, morphism_propagations},
    },
    topology::{BorderKey, MaskedTopology, RegionKey, Seam, Topology, TopologyStatistics},
};
use ahash::{HashMap, HashSet};

#[derive(Debug, Clone, Copy)]
pub enum Guess {
    Region(RegionKey),

    /// Only interior borders, the outer border is determined by the region
    InteriorBorder(BorderKey),

    /// Only seam with regions on both sides
    Seam(BorderKey, Seam),
}

impl Guess {
    pub fn variable(&self) -> Element {
        match self {
            &Guess::Region(region_key) => region_key.into(),
            &Guess::InteriorBorder(border_key) => border_key.into(),
            &Guess::Seam(_, seam) => seam.into(),
        }
    }

    pub fn guess(
        &self,
        phi: &mut Morphism,
        codom: &MaskedTopology,
        mut f: impl FnMut(&mut Morphism),
    ) {
        let tracy_span = tracy_client::span!("guess");
        tracy_span.emit_color(0xFF00FF);

        match self {
            &Self::Region(region_key) => {
                for phi_region_key in codom.visible_region_keys() {
                    phi.region_map.insert(region_key, phi_region_key);
                    // println!("Guess Region {region_key} -> Region {phi_region_key}");
                    f(phi);
                }
            }
            &Self::InteriorBorder(border_key) => {
                let phi_region_key = phi.region_map[&border_key.region_key];
                let phi_region = &codom.inner[phi_region_key];
                for i_border in 1..phi_region.boundary.borders.len() {
                    let phi_border_key = BorderKey::new(phi_region_key, i_border);
                    phi.border_map.insert(border_key, phi_border_key);
                    f(phi);
                }
            }
            &Self::Seam(border_key, seam) => {
                let phi_border_key = phi.border_map[&border_key];
                let phi_border = &codom.inner[phi_border_key];
                for phi_seam in phi_border.atomic_seams() {
                    phi.seam_map.insert(seam, phi_seam);
                    // println!("Guess {seam:?} -> {phi_seam:?}");
                    f(phi);
                }
            }
        }
    }
}

pub trait GuessChooser {
    /// Return a free seam on a border that is assigned, if possible
    fn choose_seam(
        &self,
        free: &HashSet<Element>,
        assigned: &HashSet<Element>,
        dom: &Topology,
    ) -> Option<Guess> {
        for free_variable in free {
            if let &Element::Seam(seam) = free_variable {
                if !dom.contains_seam(seam.atom_reversed()) {
                    // We only guess `seam` if `seam.reverse()` is also in `dom`.
                    continue;
                }

                let border_key = dom.seam_border(seam);
                if assigned.contains(&border_key.into()) {
                    return Some(Guess::Seam(border_key, seam));
                }
            }
        }

        None
    }

    fn choose_border(
        &self,
        free: &HashSet<Element>,
        assigned: &HashSet<Element>,
        _dom: &Topology,
    ) -> Option<Guess> {
        // Return a free broder of a region that is assigned, if possible
        for free_variable in free {
            if let &Element::Border(border_key) = free_variable {
                if assigned.contains(&border_key.region_key.into()) {
                    return Some(Guess::InteriorBorder(border_key));
                }
            }
        }

        None
    }

    // Return a free region, if possible
    fn choose_region(
        &self,
        free: &HashSet<Element>,
        _assigned: &HashSet<Element>,
        _dom: &Topology,
    ) -> Option<Guess> {
        // TODO: Return a free solid region, if possible

        for free_variable in free {
            if let &Element::Region(region_key) = free_variable {
                return Some(Guess::Region(region_key));
            }
        }

        None
    }

    /// Choose a free variable to guess we guess seams if possible, then borders, then regions.
    fn choose(
        &self,
        free: &HashSet<Element>,
        assigned: &HashSet<Element>,
        dom: &Topology,
    ) -> Option<Guess> {
        if let Some(guess) = self.choose_seam(free, assigned, dom) {
            return Some(guess);
        }

        if let Some(guess) = self.choose_border(free, assigned, dom) {
            return Some(guess);
        }

        self.choose_region(free, assigned, dom)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SimpleGuessChooser {}

impl GuessChooser for SimpleGuessChooser {}

pub struct GuessChooserUsingStatistics {
    statistics: TopologyStatistics,
}

impl GuessChooserUsingStatistics {
    pub fn new(statistics: TopologyStatistics) -> Self {
        Self { statistics }
    }
}

impl GuessChooser for GuessChooserUsingStatistics {
    // Return a free region, if possible
    fn choose_region(
        &self,
        free_vars: &HashSet<Element>,
        _assigned_vars: &HashSet<Element>,
        dom: &Topology,
    ) -> Option<Guess> {
        let free_region_vars = free_vars.iter().filter_map(|free_var| match free_var {
            &Element::Region(region_key) => Some(region_key),
            _ => None,
        });

        let lowest_material_count = free_region_vars.min_by_key(|region_key| {
            let region = &dom.regions[region_key];
            self.statistics
                .material_counts
                .get(&region.material)
                .copied()
                .unwrap_or(0)
        })?;

        Some(Guess::Region(lowest_material_count))
    }
}

pub struct GuessChooserBeginningWith<After: GuessChooser> {
    first_region: RegionKey,
    after: After,
}

impl<After: GuessChooser> GuessChooser for GuessChooserBeginningWith<After> {}

#[derive(Debug, Clone)]
pub enum ConstraintAction {
    Propagation(AnyPropagation),
    Constraint(AnyConstraint),
}

/// Search branching point where an assignment has to be guessed and tried.
#[derive(Debug, Clone)]
pub struct SearchBranch {
    guess: Guess,
    actions: Vec<ConstraintAction>,
}

pub enum SearchError {
    PropagationFailed,
    ConstraintConflict,
}

impl SearchBranch {
    #[inline(never)]
    pub fn propagate_and_check_constraints(
        &self,
        phi: &mut Morphism,
        codom: &MaskedTopology,
    ) -> Result<(), SearchError> {
        let tracy_span = tracy_client::span!("propagate_and_check_constraints");
        tracy_span.emit_color(0xFFFF00);

        for action in &self.actions {
            match action {
                ConstraintAction::Propagation(propagation) => {
                    // let tracy_span = tracy_client::span!("propagate");
                    // tracy_span.emit_color(0x00FFFF);

                    let derived = propagation.derives();
                    match propagation.derive(phi, &codom.inner) {
                        Ok(phi_derived) => {
                            // Make sure we're not assigning hidden elements
                            if let Element::Region(phi_derived) = phi_derived {
                                if codom.is_hidden(phi_derived) {
                                    return Err(SearchError::PropagationFailed);
                                }
                            }

                            phi.insert(derived, phi_derived);
                        }
                        Err(_err) => {
                            // println!("Propagation {propagation:?} failed");
                            return Err(SearchError::PropagationFailed);
                        }
                    }
                }
                ConstraintAction::Constraint(constraint) => {
                    // let tracy_span = tracy_client::span!("check_constraint");
                    // tracy_span.emit_color(0x00FFFF);

                    if !constraint.is_satisfied(phi, codom.inner) {
                        return Err(SearchError::ConstraintConflict);
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Countdown<T> {
    Alive(T, usize),
    Dead,
}

impl<T> Countdown<T> {
    pub fn decrement(&mut self) -> Option<T> {
        if let Countdown::Alive(_, count) = self {
            *count -= 1;
            if *count == 0 {
                let Countdown::Alive(inner, _) = std::mem::replace(self, Countdown::Dead) else {
                    unreachable!();
                };
                return Some(inner);
            }
            None
        } else {
            panic!("Cannot decrement when already dead!");
        }
    }
}

#[derive(Debug, Clone)]
pub struct FreeVariableTracker<T: Variables> {
    /// The list of dependents that contain the given variable
    variable_occurrences: HashMap<Element, Vec<usize>>,

    /// Dependents with number of free variables
    free_variable_counts: Vec<Countdown<T>>,

    /// Dependents with no more free variables
    closed: Vec<T>,
}

impl<T: Variables> FreeVariableTracker<T> {
    #[inline(never)]
    pub fn new(dependents: Vec<T>) -> Self {
        let mut variable_occurrences = HashMap::default();
        let mut free_variable_counts = Vec::new();
        for (i_dependent, dependent) in dependents.into_iter().enumerate() {
            for variable in dependent.variables() {
                variable_occurrences
                    .entry(variable.clone())
                    .or_insert(Vec::new())
                    .push(i_dependent);
            }
            let variable_count = dependent.variables().len();
            free_variable_counts.push(Countdown::Alive(dependent, variable_count));
        }

        Self {
            variable_occurrences,
            free_variable_counts,
            closed: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.free_variable_counts.len()
    }

    pub fn assign_variable(&mut self, variable: &Element) {
        let Some(variable_occurrences) = self.variable_occurrences.get(variable) else {
            return;
        };

        for &i_dependent in variable_occurrences {
            if let Some(dependent) = self.free_variable_counts[i_dependent].decrement() {
                self.closed.push(dependent);
            }
        }
    }

    pub fn pop_closed(&mut self) -> Option<T> {
        self.closed.pop()
    }
}

#[derive(Debug, Clone)]
pub struct ConstraintSystem {
    pub variables: HashSet<Element>,

    pub constraints: FreeVariableTracker<AnyConstraint>,

    pub propagations: FreeVariableTracker<AnyPropagation>,
}

impl ConstraintSystem {
    #[inline(never)]
    pub fn variables(dom: &Topology) -> HashSet<Element> {
        let mut variables = HashSet::default();

        for (&region_key, region) in &dom.regions {
            variables.insert(region_key.into());

            for (i_border, border) in region.boundary.borders.iter().enumerate() {
                let border_key = BorderKey::new(region_key, i_border);
                variables.insert(border_key.into());

                for seam in border.atomic_seams() {
                    variables.insert(seam.into());
                }

                for corner in border.corners() {
                    variables.insert(corner.into());
                }
            }
        }

        variables
    }

    #[inline(never)]
    pub fn for_morphism(dom: &Topology) -> Self {
        Self {
            constraints: FreeVariableTracker::new(morphism_constraints(dom)),
            propagations: FreeVariableTracker::new(morphism_propagations(dom)),
            variables: Self::variables(dom),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchPlan {
    pub steps: Vec<SearchBranch>,
}

impl SearchPlan {
    /// Make a plan to find solutions. Using `first` one can fix the guess that the plan should
    /// start with, otherwise `guess_chooser` is used to find the initial guess.
    #[inline(never)]
    pub fn new(
        constraint_system: ConstraintSystem,
        dom: &Topology,
        guess_chooser: &impl GuessChooser,
        first: Option<Guess>,
    ) -> Self {
        let ConstraintSystem {
            variables: mut free_variables,
            mut constraints,
            mut propagations,
        } = constraint_system;

        let tracy_span = tracy_client::span!("SearchPlan::for_morphism");

        let mut assigned_variables: HashSet<Element> = HashSet::default();

        let mut steps = Vec::new();

        while !free_variables.is_empty() {
            let guess = if let Some(first) = first
                && assigned_variables.is_empty()
            {
                first
            } else {
                assert!(!free_variables.is_empty());
                guess_chooser
                    .choose(&free_variables, &assigned_variables, dom)
                    .unwrap()
            };

            free_variables.remove(&guess.variable());
            assigned_variables.insert(guess.variable());
            propagations.assign_variable(&guess.variable());
            constraints.assign_variable(&guess.variable());

            // Check if there are any applicable constraints or propagations
            let mut actions = Vec::new();
            loop {
                if let Some(constraint) = constraints.pop_closed() {
                    debug_assert!(
                        constraint
                            .variables()
                            .iter()
                            .all(|variable| assigned_variables.contains(variable))
                    );

                    let action = ConstraintAction::Constraint(constraint);
                    actions.push(action);
                    continue;
                }

                if let Some(propagation) = propagations.pop_closed() {
                    debug_assert!(
                        propagation
                            .variables()
                            .iter()
                            .all(|variable| assigned_variables.contains(variable))
                    );

                    let derived = propagation.derives();
                    let was_free = free_variables.remove(&derived);
                    if was_free {
                        propagations.assign_variable(&derived);
                        constraints.assign_variable(&derived);
                        assigned_variables.insert(derived);

                        let action = ConstraintAction::Propagation(propagation);
                        actions.push(action);
                    }

                    continue;
                }

                // No applicable constraints or propagations left, so we need to make another guess.
                break;
            }

            let step = SearchBranch { guess, actions };
            steps.push(step);
        }

        // Statistics for tracy region
        {
            let mut n_regions = 0;
            let mut n_seams = 0;
            let mut n_corners = 0;
            let mut n_borders = 0;
            for var in &free_variables {
                match var {
                    Element::Seam(_) => {
                        n_seams += 1;
                    }
                    Element::Corner(_) => {
                        n_corners += 1;
                    }
                    Element::Region(_) => {
                        n_regions += 1;
                    }
                    Element::Border(_) => {
                        n_borders += 1;
                    }
                }
            }
            let n_constraints = constraints.len();
            let n_propagations = propagations.len();

            let n_steps = steps.len();
            tracy_span.emit_text(&format!(
                "regions: {n_regions}, seams: {n_seams}, corners: {n_corners}, borders: {n_borders} \
            constraints: {n_constraints}, propagations: {n_propagations}, steps: {n_steps}",
            ));
        }

        assert!(free_variables.is_empty());
        // assert!(constraints.is_empty());

        Self { steps }
    }

    // pub fn for_morphism(
    //     dom: &Topology,
    //     guess_chooser: &impl GuessChooser,
    //     first: Option<Guess>,) -> Vec<(Material, Self)> {
    //
    // }

    #[inline(never)]
    pub fn search_step(
        &self,
        i_step: usize,
        phi: &mut Morphism,
        codom: &MaskedTopology,
        found: &mut impl FnMut(&Morphism),
    ) {
        let tracy_span = tracy_client::span!("search_step");
        tracy_span.emit_color(0xFFFF00);

        if i_step >= self.steps.len() {
            found(phi);
            return;
        }

        let step = &self.steps[i_step];

        step.guess.guess(phi, codom, |phi| {
            if step.propagate_and_check_constraints(phi, codom).is_err() {
                return;
            }

            self.search_step(i_step + 1, phi, codom, found);
        })
    }

    #[inline(never)]
    pub fn search(&self, codom: &MaskedTopology, mut found: impl FnMut(&Morphism)) {
        let mut phi = Morphism::new();
        self.search_step(0, &mut phi, codom, &mut found);
    }

    #[inline(never)]
    pub fn search_with_first_guessed(
        &self,
        codom: &MaskedTopology,
        phi_region_key: RegionKey,
        mut found: impl FnMut(&Morphism),
    ) {
        let tracy_span = tracy_client::span!("search_with_first_guessed");
        tracy_span.emit_color(0xFFFF00);

        let first_step = self.steps.first().unwrap();

        // Create Morphism with first guess assigned
        let mut phi = Morphism::new();
        let Guess::Region(region_key) = first_step.guess else {
            panic!("First guess must be region");
        };
        phi.region_map.insert(region_key, phi_region_key);

        if first_step
            .propagate_and_check_constraints(&mut phi, codom)
            .is_err()
        {
            return;
        }

        self.search_step(1, &mut phi, codom, &mut found);
    }

    #[inline(never)]
    pub fn solutions(&self, codom: &MaskedTopology) -> Vec<Morphism> {
        let mut solutions = Vec::new();
        self.search(codom, |phi| {
            solutions.push(phi.clone());
        });
        solutions
    }

    pub fn print(&self) {
        // Print plan
        for (i_step, step) in self.steps.iter().enumerate() {
            println!("Step {i_step}");
            println!("  Guess {:?}", &step.guess);
            println!("  Actions");
            for action in &step.actions {
                match action {
                    ConstraintAction::Propagation(propagation) => {
                        println!(
                            "    Propagation {:?} -> {:?}",
                            propagation.as_propagation(),
                            propagation.derives()
                        );
                    }
                    ConstraintAction::Constraint(constraint) => {
                        println!("    Constraint {:?}", constraint.as_constraint());
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchStrategy {
    pub plans: Vec<(Material, SearchPlan)>,
    pub main_plan: SearchPlan,
}

impl SearchStrategy {
    #[inline(never)]
    pub fn for_morphism(dom: &Topology, guess_chooser: &impl GuessChooser) -> Self {
        let _tracy_span = tracy_client::span!("SearchStrategy::for_morphism");

        let constraint_system = ConstraintSystem::for_morphism(dom);
        let mut plans = Vec::new();
        for region_key in dom.iter_region_keys() {
            let material = dom[region_key].material;
            let first_guess = Guess::Region(region_key);

            let plan = SearchPlan::new(
                constraint_system.clone(),
                dom,
                guess_chooser,
                Some(first_guess),
            );
            plans.push((material, plan));
        }

        let main_plan = SearchPlan::new(constraint_system, dom, guess_chooser, None);

        Self { plans, main_plan }
    }

    /// Find all solutions `phi` where the image of `phi` contains `region_key`
    #[inline(never)]
    pub fn solutions(&self, codom: &MaskedTopology, contained: Option<RegionKey>) -> Vec<Morphism> {
        let _span = tracy_client::span!("SearchStrategy::solutions");

        let mut solutions = Vec::new();
        // Without the explicit type annotation of the lambda fails to compile, weird. Maybe the
        // lifetime it derives for phi is wrong.
        let mut on_solution_found = |phi: &Morphism| {
            for &phi_region_key in phi.region_map.values() {
                assert!(!codom.is_hidden(phi_region_key));
            }

            solutions.push(phi.clone());
        };

        if let Some(contained) = contained {
            // If `contained` is hidden there are no solutions
            if codom.is_hidden(contained) {
                return Vec::new();
            }

            let region = &codom.inner[contained];
            for pair in &self.plans {
                let (first_material, plan) = pair;
                if first_material.matches(region.material) {
                    plan.search_with_first_guessed(codom, contained, &mut on_solution_found)
                }
            }
        } else {
            // self.main_plan.search(codom, |phi| {

            // })
            self.main_plan.search(codom, &mut on_solution_found)
        }

        solutions
    }
}

#[cfg(test)]
mod test {
    use crate::{
        field::RgbaField,
        material::Material,
        math::rgba8::Rgba8,
        pixmap::MaterialMap,
        solver::plan::{ConstraintSystem, SearchPlan, SimpleGuessChooser},
        topology::{MaskedTopology, Topology},
    };
    use itertools::Itertools;
    use std::path::Path;

    fn load(path: impl AsRef<Path>) -> Topology {
        let rgba_field = RgbaField::load(path).unwrap();
        let material_map = MaterialMap::from(rgba_field).without(Material::RULE_BEFORE);
        Topology::new(&material_map)
    }

    #[inline(never)]
    pub fn extract_pattern(material_map: &mut MaterialMap) -> MaterialMap {
        let topo = Topology::new(material_map);

        const PATTERN_FRAME_MATERIAL: Material = Material::normal(Rgba8::MAGENTA.rgb());

        let frame = topo
            .regions
            .values()
            .find(|&region| region.material == PATTERN_FRAME_MATERIAL)
            .unwrap();
        assert_eq!(frame.boundary.borders.len(), 2);

        let inner_border = frame
            .boundary
            .borders
            .iter()
            .find(|border| !border.is_outer)
            .unwrap();

        // Extract right of border
        let right_of_border = material_map.right_of_border(inner_border);
        for pixel in right_of_border.keys() {
            material_map.remove(pixel);
        }

        right_of_border
    }

    fn load_material_map(path: impl AsRef<Path>) -> MaterialMap {
        let rgba_field = RgbaField::load(path).unwrap();
        MaterialMap::from(rgba_field).without(Material::RULE_BEFORE)
    }

    fn assert_extract_inner_outer(name: &str) {
        let folder = "test_resources/extract_pattern";
        let mut pixmap = RgbaField::load(format!("{folder}/{name}.png"))
            .unwrap()
            .into();
        let inner = extract_pattern(&mut pixmap);

        // Load expected inner and outer pixmaps
        let expected_inner = load_material_map(format!("{folder}/{name}_inner.png"));
        assert!(inner.defined_equals(&expected_inner));

        let expected_outer = load_material_map(format!("{folder}/{name}_outer.png"));
        // pixmap.save(format!("{folder}/{name}_outer_actual.png")).unwrap();
        assert!(pixmap.defined_equals(&expected_outer));
    }

    fn assert_solve(dom_filename: &str, codom_filename: &str, expected_solutions_len: usize) {
        let folder = "test_resources/patterns/";

        let dom = load(format!("{folder}/{dom_filename}"));
        let codom = load(format!("{folder}/{codom_filename}"));

        let guess_chooser = SimpleGuessChooser::default();
        let constraint_system = ConstraintSystem::for_morphism(&dom);
        let plan = SearchPlan::new(constraint_system, &dom, &guess_chooser, None);
        // plan.print();

        let solutions = plan.solutions(&MaskedTopology::whole(&codom));

        for phi in &solutions {
            assert!(phi.is_homomorphism(&dom, &codom));
            // println!("{phi}");
        }

        // Check if there are duplicate solutions
        let distinct_solutions: Vec<_> = solutions.iter().unique().collect();
        assert_eq!(distinct_solutions.len(), solutions.len());

        assert_eq!(solutions.len(), expected_solutions_len);
    }

    #[test]
    fn extract_pattern_a() {
        assert_extract_inner_outer("a");
    }

    #[test]
    fn extract_pattern_b() {
        assert_extract_inner_outer("b");
    }

    #[test]
    fn extract_pattern_c() {
        assert_extract_inner_outer("c");
    }

    #[test]
    fn pattern_matches_a() {
        assert_solve("a/pattern.png", "a/match_1.png", 1);
        assert_solve("a/pattern.png", "a/match_2.png", 1);
        assert_solve("a/pattern.png", "a/match_3.png", 1);
        assert_solve("a/pattern.png", "a/match_4.png", 3);
        assert_solve("a/pattern.png", "a/miss_1.png", 0);
    }

    #[test]
    fn pattern_matches_b() {
        assert_solve("b/pattern.png", "b/match_1.png", 1);
        assert_solve("b/pattern.png", "b/match_2.png", 1);
        assert_solve("b/pattern.png", "b/match_3.png", 2);
        assert_solve("b/pattern.png", "b/match_4.png", 4);
        assert_solve("b/pattern.png", "b/miss_1.png", 0);
    }

    #[test]
    fn pattern_matches_c() {
        assert_solve("c/pattern.png", "c/match_1.png", 1);
        assert_solve("c/pattern.png", "c/miss_1.png", 0);
        assert_solve("c/pattern.png", "c/miss_2.png", 0);
    }

    #[test]
    fn pattern_matches_single_hole() {
        assert_solve("single_hole/pattern.png", "single_hole/miss_1.png", 0);
        assert_solve("single_hole/pattern.png", "single_hole/match_2.png", 1);
        assert_solve("single_hole/pattern.png", "single_hole/match_3.png", 1);
    }

    #[test]
    fn pattern_matches_interior_hole() {
        assert_solve("interior_hole/pattern.png", "interior_hole/miss_1.png", 0);
        assert_solve("interior_hole/pattern.png", "interior_hole/miss_2.png", 0);
        assert_solve("interior_hole/pattern.png", "interior_hole/match_1.png", 1);
        assert_solve("interior_hole/pattern.png", "interior_hole/match_2.png", 1);
        assert_solve("interior_hole/pattern.png", "interior_hole/match_3.png", 1);
        assert_solve("interior_hole/pattern.png", "interior_hole/match_4.png", 1);
    }

    #[test]
    fn pattern_matches_two_holes_a() {
        assert_solve("two_holes_a/pattern.png", "two_holes_a/match_1.png", 2);
    }

    #[test]
    fn pattern_matches_two_holes_b() {
        assert_solve("two_holes_b/pattern.png", "two_holes_b/match_1.png", 1);
    }

    #[test]
    fn pattern_matches_two_holes_c() {
        assert_solve("two_holes_c/pattern.png", "two_holes_c/match_1.png", 1);
        assert_solve("two_holes_c/pattern.png", "two_holes_c/miss_1.png", 0);
    }

    #[test]
    fn pattern_matches_gate_a() {
        assert_solve("gate_a/pattern.png", "gate_a/match_1.png", 1);
        assert_solve("gate_a/pattern.png", "gate_a/match_2.png", 1);
        assert_solve("gate_a/pattern.png", "gate_a/match_3.png", 1);
    }

    #[test]
    fn pattern_matches_rule_frame_a() {
        assert_solve("rule_frame_a/pattern.png", "rule_frame_a/match_1.png", 1);
        assert_solve("rule_frame_a/pattern.png", "rule_frame_a/match_2.png", 1);
        assert_solve("rule_frame_a/pattern.png", "rule_frame_a/match_3.png", 1);
        assert_solve("rule_frame_a/pattern.png", "rule_frame_a/match_4.png", 1);
    }

    #[test]
    fn pattern_matches_rule_frame_b() {
        assert_solve("rule_frame_b/pattern.png", "rule_frame_b/match_1.png", 1);
        assert_solve("rule_frame_b/pattern.png", "rule_frame_b/match_2.png", 1);
        assert_solve("rule_frame_b/pattern.png", "rule_frame_b/match_3.png", 1);
        assert_solve("rule_frame_b/pattern.png", "rule_frame_b/match_4.png", 1);
    }

    #[test]
    fn pattern_matches_solid_a() {
        assert_solve("solid/a/pattern.png", "solid/a/match_1.png", 1);
        assert_solve("solid/a/pattern.png", "solid/a/miss_1.png", 0);
        assert_solve("solid/a/pattern.png", "solid/a/miss_2.png", 0);
        assert_solve("solid/a/pattern.png", "solid/a/miss_3.png", 0);
        assert_solve("solid/a/pattern.png", "solid/a/miss_4.png", 0);
    }

    #[test]
    fn pattern_matches_solid_b() {
        assert_solve("solid/b/pattern.png", "solid/b/match_1.png", 1);
        assert_solve("solid/b/pattern.png", "solid/b/miss_1.png", 0);
    }

    #[test]
    fn pattern_matches_solid_c() {
        assert_solve("solid/c/pattern.png", "solid/c/match_1.png", 1);
        assert_solve("solid/c/pattern.png", "solid/c/miss_1.png", 0);
    }

    #[test]
    fn pattern_matches_solid_symbol() {
        assert_solve("solid/symbol/pattern.png", "solid/symbol/match_1.png", 1);
        assert_solve("solid/symbol/pattern.png", "solid/symbol/match_2.png", 1);
    }

    #[test]
    fn pattern_matches_solid_void() {
        assert_solve("solid/void/pattern.png", "solid/void/match_1.png", 1);
        assert_solve("solid/void/pattern.png", "solid/void/match_2.png", 1);
        assert_solve("solid/void/pattern.png", "solid/void/miss_1.png", 0);
    }

    #[test]
    fn wildcard_a() {
        assert_solve("wildcard_a/pattern.png", "wildcard_a/match_1.png", 1);
        assert_solve("wildcard_a/pattern.png", "wildcard_a/match_2.png", 1);
        assert_solve("wildcard_a/pattern.png", "wildcard_a/miss_1.png", 0);
        assert_solve("wildcard_a/pattern.png", "wildcard_a/miss_2.png", 0);
        assert_solve("wildcard_a/pattern.png", "wildcard_a/miss_3.png", 0);
        assert_solve("wildcard_a/pattern.png", "wildcard_a/miss_4.png", 0);
    }

    #[test]
    fn wildcard_b() {
        assert_solve("wildcard_b/pattern.png", "wildcard_b/match_1.png", 1);
        assert_solve("wildcard_b/pattern.png", "wildcard_b/miss_1.png", 0);
        assert_solve("wildcard_b/pattern.png", "wildcard_b/miss_2.png", 0);
    }

    #[test]
    fn wildcard_c() {
        assert_solve("wildcard_c/pattern.png", "wildcard_c/match_1.png", 1);
        assert_solve("wildcard_c/pattern.png", "wildcard_c/match_2.png", 1);
        assert_solve("wildcard_c/pattern.png", "wildcard_c/match_3.png", 2);
        // Not 100% sure this is the right answer
        assert_solve("wildcard_c/pattern.png", "wildcard_c/match_4.png", 2);

        assert_solve("wildcard_c/pattern.png", "wildcard_c/miss_1.png", 0);
        assert_solve("wildcard_c/pattern.png", "wildcard_c/miss_2.png", 0);
    }

    #[test]
    fn disjoint() {
        assert_solve("disjoint/pattern.png", "disjoint/match_1.png", 1);
        assert_solve("disjoint/pattern.png", "disjoint/match_2.png", 2);
        assert_solve("disjoint/pattern.png", "disjoint/match_3.png", 4);
    }
}
