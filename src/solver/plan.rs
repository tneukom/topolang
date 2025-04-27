use crate::{
    material::Material,
    morphism::Morphism,
    solver::{
        constraints::{morphism_constraints, AnyConstraint, Constraint},
        element::Element,
        propagations::{morphism_propagations, AnyPropagation, Propagation},
    },
    topology::{BorderKey, RegionKey, Seam, StrongRegionKey, Topology, TopologyStatistics},
};
use ahash::HashSet;
use std::collections::BTreeSet;

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

    pub fn guess(&self, phi: &mut Morphism, codom: &Topology, mut f: impl FnMut(&mut Morphism)) {
        match self {
            &Self::Region(region_key) => {
                for &phi_region_key in codom.regions.keys() {
                    phi.region_map.insert(region_key, phi_region_key);
                    // println!("Guess Region {region_key} -> Region {phi_region_key}");
                    f(phi);
                }
            }
            &Self::InteriorBorder(border_key) => {
                let phi_region_key = phi.region_map[&border_key.region_key];
                let phi_region = &codom[phi_region_key];
                for i_border in 1..phi_region.boundary.borders.len() {
                    let phi_border_key = BorderKey::new(phi_region_key, i_border);
                    phi.border_map.insert(border_key, phi_border_key);
                    f(phi);
                }
            }
            &Self::Seam(border_key, seam) => {
                let phi_border_key = phi.border_map[&border_key];
                let phi_border = &codom[phi_border_key];
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
pub struct SearchStep {
    guess: Guess,
    propagations: Vec<AnyPropagation>,
    constraints: Vec<AnyConstraint>,
}

pub enum SearchError {
    PropagationFailed,
    ConstraintConflict,
}

impl SearchStep {
    #[inline(never)]
    pub fn propagate(&self, phi: &mut Morphism, codom: &Topology) -> Result<(), SearchError> {
        for propagation in &self.propagations {
            let derived = propagation.derives();
            match propagation.derive(phi, codom) {
                Ok(phi_derived) => {
                    // println!(
                    //     "Propagated by {} {:?} -> {:?}",
                    //     propagation.name(),
                    //     derived,
                    //     phi_derived
                    // );
                    phi.insert(derived, phi_derived);
                }
                Err(_err) => {
                    // println!("Propagation {propagation:?} failed");
                    return Err(SearchError::PropagationFailed);
                }
            }
        }

        Ok(())
    }

    #[inline(never)]
    pub fn check_constraints(&self, phi: &Morphism, codom: &Topology) -> Result<(), SearchError> {
        for constraint in &self.constraints {
            if !constraint.is_satisfied(phi, codom) {
                return Err(SearchError::ConstraintConflict);
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SearchPlan {
    steps: Vec<SearchStep>,
}

impl SearchPlan {
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

    /// Make a plan to find solutions. Using `first` one can fix the guess that the plan should
    /// start with, otherwise `guess_chooser` is used to find the initial guess.
    pub fn for_morphism(
        dom: &Topology,
        guess_chooser: &impl GuessChooser,
        first: Option<Guess>,
    ) -> Self {
        let mut available_constraints = morphism_constraints(dom);
        let available_propagations = morphism_propagations(dom);
        // for propagation in &propagations {
        //     println!("Available propagation: {propagation:?}");
        // }

        // All elements (regions, borders, seams, corners) that appear in dom
        let mut free_variables: HashSet<Element> = Self::variables(dom);
        let mut assigned_variables: HashSet<Element> = HashSet::default();

        let mut steps = Vec::new();

        while !free_variables.is_empty() {
            // TODO: Use let-chain when stable
            let guess = if assigned_variables.is_empty() && first.is_some() {
                first.unwrap()
            } else {
                guess_chooser
                    .choose(&free_variables, &assigned_variables, dom)
                    .unwrap()
            };

            free_variables.remove(&guess.variable());
            assigned_variables.insert(guess.variable());

            // Propagate any variable we can
            let mut propagations = Vec::new();
            loop {
                // Find a propagated to apply or break if there is none.
                let Some(applicable) = available_propagations.iter().find(|propagation| {
                    if assigned_variables.contains(&propagation.derives()) {
                        return false;
                    }

                    propagation
                        .require()
                        .iter()
                        .all(|required| assigned_variables.contains(required))
                }) else {
                    break;
                };

                let derived = applicable.derives();
                free_variables.remove(&derived);
                assigned_variables.insert(derived);
                propagations.push(applicable.clone())
            }

            // Check any constraint we can
            // TODO: Use extract_if when stable (https://github.com/rust-lang/rust/issues/43244)
            let mut constraints = Vec::new();
            let mut i = 0;
            while i < available_constraints.len() {
                let constraint = &available_constraints[i];
                let applicable = constraint
                    .variables()
                    .iter()
                    .all(|variable| assigned_variables.contains(variable));
                if applicable {
                    let constraint = available_constraints.remove(i);
                    constraints.push(constraint);
                } else {
                    i += 1;
                }
            }

            let step = SearchStep {
                guess,
                propagations,
                constraints,
            };
            steps.push(step);
        }

        assert!(available_constraints.is_empty());
        assert!(free_variables.is_empty());

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
        codom: &Topology,
        found: &mut impl FnMut(&Morphism),
    ) {
        // println!("Step {i_step}");

        if i_step >= self.steps.len() {
            found(phi);
            return;
        }

        let step = &self.steps[i_step];

        step.guess.guess(phi, codom, |phi| {
            if step.propagate(phi, codom).is_err() {
                return;
            }

            if step.check_constraints(phi, codom).is_err() {
                return;
            }

            self.search_step(i_step + 1, phi, codom, found);
        })
    }

    #[inline(never)]
    pub fn search(&self, codom: &Topology, mut found: impl FnMut(&Morphism)) {
        let mut phi = Morphism::new();
        self.search_step(0, &mut phi, codom, &mut found);
    }

    #[inline(never)]
    pub fn search_with_first_guessed(
        &self,
        codom: &Topology,
        phi_region_key: RegionKey,
        mut found: impl FnMut(&Morphism),
    ) {
        let first_step = self.steps.first().unwrap();

        // Create Morphism with first guess assigned
        let mut phi = Morphism::new();
        let Guess::Region(region_key) = first_step.guess else {
            panic!("First guess must be region");
        };
        phi.region_map.insert(region_key, phi_region_key);

        if first_step.propagate(&mut phi, codom).is_err() {
            return;
        }

        if first_step.check_constraints(&phi, codom).is_err() {
            return;
        }

        self.search_step(1, &mut phi, codom, &mut found);
    }

    #[inline(never)]
    pub fn solutions(&self, codom: &Topology) -> Vec<Morphism> {
        let mut solutions = Vec::new();
        self.search(codom, |phi| {
            solutions.push(phi.clone());
        });
        solutions
    }

    fn is_excluded(
        phi: &Morphism,
        codom: &Topology,
        excluding: &BTreeSet<StrongRegionKey>,
    ) -> bool {
        phi.region_map.values().any(|phi_region_key| {
            let strong_phi_region_key = codom.regions[phi_region_key].top_left_interior_pixel();
            excluding.contains(&strong_phi_region_key)
        })
    }

    #[inline(never)]
    pub fn solutions_excluding(
        &self,
        codom: &Topology,
        excluding: &BTreeSet<StrongRegionKey>,
    ) -> Vec<Morphism> {
        let mut solutions = Vec::new();
        self.search(codom, |phi| {
            if !Self::is_excluded(phi, codom, excluding) {
                solutions.push(phi.clone());
            }
        });
        solutions
    }

    #[inline(never)]
    pub fn first_solution(&self, codom: &Topology) -> Option<Morphism> {
        // TODO: Abort search after first solution found
        self.solutions(codom).into_iter().next()
    }

    pub fn print(&self) {
        // Print plan
        for (i_step, step) in self.steps.iter().enumerate() {
            println!("Step {i_step}");
            println!("  Guess {:?}", &step.guess);
            println!("  Propagations");
            for propagation in &step.propagations {
                println!(
                    "    {:?} -> {:?}",
                    propagation.as_propagation(),
                    propagation.derives()
                );
            }
            println!("  Constraints");
            for constraint in &step.constraints {
                println!("    {:?}", constraint.as_constraint());
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
    pub fn for_morphism(dom: &Topology, guess_chooser: &impl GuessChooser) -> Self {
        let mut plans = Vec::new();
        for region_key in dom.iter_region_keys() {
            let material = dom[region_key].material;
            let first_guess = Guess::Region(region_key);
            let plan = SearchPlan::for_morphism(dom, guess_chooser, Some(first_guess));
            plans.push((material, plan));
        }

        let main_plan = SearchPlan::for_morphism(dom, guess_chooser, None);

        Self { plans, main_plan }
    }

    /// Find all solutions `phi` where the image of `phi` contains `region_key` but does not
    /// contain any element in `excluded`.
    #[inline(never)]
    pub fn solutions(
        &self,
        codom: &Topology,
        contained: Option<RegionKey>,
        excluded: &BTreeSet<StrongRegionKey>,
    ) -> Vec<Morphism> {
        let mut solutions = Vec::new();
        // Without the explicit type annotation of the lambda fails to compile, weird. Maybe the
        // lifetime it derives for phi is wrong.
        let mut on_solution_found = |phi: &Morphism| {
            if !SearchPlan::is_excluded(phi, codom, excluded) {
                solutions.push(phi.clone());
            }
        };

        if let Some(contained) = contained {
            let region = &codom[contained];
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
        solver::plan::{SearchPlan, SimpleGuessChooser},
        topology::Topology,
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
        let plan = SearchPlan::for_morphism(&dom, &guess_chooser, None);
        // plan.print();

        let solutions = plan.solutions(&codom);

        println!("Number of solutions found: {}", solutions.len());
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
