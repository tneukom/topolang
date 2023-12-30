use crate::{
    math::{pixel::Vertex, rgba8::Rgba8},
    morphism::Morphism,
    pixmap::Pixmap,
    topology::{RegionKey, Seam, Topology},
    utils::find_duplicate_by,
};
use std::{cmp::Ordering, collections::BTreeMap};

pub fn generalized_seams(topo: &Topology) -> Vec<Seam> {
    let mut seams = Vec::new();
    for border in topo.iter_borders() {
        for i in 0..border.seams.len() {
            // All seams that go around exactly once are equivalent, so we only include one.
            let len = if i == 0 {
                // Include seam that goes around the border.
                border.seams.len()
            } else {
                // Skip seam that goes around.
                border.seams.len() - 1
            };

            for j in 0..len {
                let start = border.seams[i].start;
                let stop = border.seams[(i + j) % border.seams.len()].stop;
                let seam = Seam::new_with_len(start, stop, j + 1);
                seams.push(seam)
            }
        }
    }

    let duplicate_seam = find_duplicate_by(&seams, |lhs, rhs| topo.seams_equivalent(lhs, rhs));
    assert!(duplicate_seam.is_none());
    seams
}

#[derive(Debug, Clone, Copy)]
pub struct CoutTrace {
    level: usize,
}

impl CoutTrace {
    pub fn new() -> Self {
        Self { level: 0 }
    }

    pub fn assign(&self, seam: &Seam, phi_seam: &Seam) {
        let indent = self.indent();
        println!("{indent}{seam} -> {phi_seam}")
    }

    pub fn failed(&self) {
        let indent = self.indent();
        println!("{indent}Failed")
    }

    pub fn success(&self) {
        let indent = self.indent();
        println!("{indent}Succeeded")
    }

    pub fn indent(&self) -> String {
        "  ".repeat(self.level)
    }

    pub fn recurse(&self) -> Self {
        Self {
            level: self.level + 1,
        }
    }
}

pub trait Trace {
    fn assign(&self, seam: &Seam, phi_seam: &Seam);

    fn failed(&self, cause: &str);

    fn success(&self);

    fn recurse(&self) -> Self;
}

impl Trace for CoutTrace {
    fn assign(&self, seam: &Seam, phi_seam: &Seam) {
        let indent = self.indent();
        println!("{indent}{seam} -> {phi_seam}")
    }

    fn failed(&self, cause: &str) {
        let indent = self.indent();
        println!("{indent}Failed {cause}")
    }

    fn success(&self) {
        let indent = self.indent();
        println!("{indent}Succeeded")
    }

    fn recurse(&self) -> Self {
        Self {
            level: self.level + 1,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NullTrace {}

impl NullTrace {
    pub fn new() -> Self {
        NullTrace {}
    }
}

impl Trace for NullTrace {
    fn assign(&self, _seam: &Seam, _phi_seam: &Seam) {}

    fn failed(&self, _cause: &str) {}

    fn success(&self) {}

    fn recurse(&self) -> Self {
        *self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unassigned {
    seam: Seam,
    phi_left: Option<RegionKey>,
    phi_start_corner: Option<Vertex>,
    phi_stop_corner: Option<Vertex>,
    reverse_in_pattern: bool,
}

impl Unassigned {
    /// List of unassigned seams given a pattern and a partial Morphism `phi`
    /// If returned list is empty phi is fully defined
    pub fn candidates(pattern: &Topology, phi: &Morphism) -> Vec<Self> {
        pattern
            .iter_seams()
            .filter(|&seam| !phi.seam_map.contains_key(seam))
            .map(|seam| Self {
                seam: *seam,
                phi_left: phi.region_map.get(&pattern.left_of(seam)).copied(),
                phi_start_corner: phi.corner_map.get(&seam.start_corner()).copied(),
                phi_stop_corner: phi.corner_map.get(&seam.stop_corner()).copied(),
                reverse_in_pattern: pattern.contains_seam(&seam.reversed()),
            })
            .collect()
    }

    /// lhs > rhs means lhs should be assigned before rhs because it is more restricted
    /// by the already assigned seams and/or restricts other items more than rhs.
    /// Not a total order (not antisymmetric)
    fn compare_heuristic(&self, other: &Self) -> Ordering {
        let cmp_tuple = |unassigned: &Unassigned| {
            (
                unassigned.phi_left.is_some(),
                unassigned.phi_start_corner.is_some(),
                unassigned.phi_stop_corner.is_some(),
                unassigned.reverse_in_pattern,
            )
        };

        cmp_tuple(self).cmp(&cmp_tuple(other))
    }

    /// Choose the best unassigned seam to continue the search
    pub fn choose(pattern: &Topology, phi: &Morphism) -> Option<Self> {
        Self::candidates(pattern, phi)
            .into_iter()
            .max_by(Self::compare_heuristic)
    }

    pub fn possible_assignment(
        &self,
        world: &Topology,
        pattern: &Topology,
        phi_seam: &Seam,
    ) -> bool {
        if pattern[pattern.left_of(&self.seam)].color != world[world.left_of(phi_seam)].color {
            // Inconsistent left side color
            return false;
        }

        if self.reverse_in_pattern {
            if !phi_seam.is_atom() {
                return false;
            }

            let Some(right_of_phi_seam) = world.right_of(phi_seam) else {
                // phi_seam is not reversible in world
                return false;
            };

            if world[right_of_phi_seam].color
                != pattern[pattern.right_of(&self.seam).unwrap()].color
            {
                // Inconsistent right color
                return false;
            }
        }

        if let Some(phi_left) = self.phi_left {
            if phi_left != world.left_of(phi_seam) {
                // inconsistent left side region
                return false;
            }
        }

        if let Some(phi_start_corner) = self.phi_start_corner {
            if phi_start_corner != phi_seam.start_corner() {
                // inconsistent start corner
                return false;
            }
        }

        if let Some(phi_stop_corner) = self.phi_stop_corner {
            if phi_stop_corner != phi_seam.stop_corner() {
                // inconsistent stop corner
                return false;
            }
        }

        true
    }

    /// Returns possible candidate seams in `world` that `self.seam` could be mapped to.
    /// Returned seam don't have to be atomic.
    pub fn assignment_candidates(&self, world: &Topology, pattern: &Topology) -> Vec<Seam> {
        generalized_seams(world)
            .iter()
            .filter(|&phi_seam| self.possible_assignment(world, pattern, phi_seam))
            .copied()
            .collect()
    }
}

pub fn search_step(
    world: &Topology,
    pattern: &Topology,
    partial: BTreeMap<Seam, Seam>,
    solutions: &mut Vec<Morphism>,
    trace: impl Trace,
) {
    // Check if partial assignment is consistent
    let Some(phi) = Morphism::induced_from_seam_map(pattern, world, partial.clone()) else {
        trace.failed("Partial assignment inconsistent");
        return;
    };

    if !phi.is_homomorphism(pattern, world) || !phi.is_injective() {
        trace.failed("Morphism inconsistent");
        return;
    }

    // Find a seam that is not yet assigned
    let unassigned = Unassigned::choose(pattern, &phi);
    let Some(unassigned) = unassigned else {
        // If seams are assigned, check morphism is proper
        trace.success();
        solutions.push(phi);
        return;
    };

    let assignment_candidates = unassigned.assignment_candidates(world, pattern);
    if assignment_candidates.is_empty() {
        trace.failed("No assignment candidates");
        return;
    }

    // Assign the found seam to all possible candidates and recurse
    for phi_unassigned in assignment_candidates {
        let mut ext_partial = partial.clone();
        ext_partial.insert(unassigned.seam, phi_unassigned);
        trace.assign(&unassigned.seam, &phi_unassigned);
        search_step(world, pattern, ext_partial, solutions, trace.recurse())
    }
}

pub fn find_matches(world: &Topology, pattern: &Topology, trace: impl Trace) -> Vec<Morphism> {
    let mut solutions = Vec::new();
    let partial = BTreeMap::new();
    search_step(world, pattern, partial, &mut solutions, trace);
    solutions
}

const PATTERN_FRAME_COLOR: Rgba8 = Rgba8::MAGENTA;

pub fn extract_pattern(pixmap: &mut Pixmap) -> Pixmap {
    let topo = Topology::new(&pixmap);

    let frame = topo
        .regions
        .values()
        .find(|&region| region.color == PATTERN_FRAME_COLOR)
        .unwrap();
    assert_eq!(frame.boundary.len(), 2);

    let inner_border = frame
        .boundary
        .iter()
        .find(|border| !border.is_outer)
        .unwrap();
    pixmap.extract_right(inner_border)
}

#[cfg(test)]
mod test {
    use crate::{
        math::rgba8::Rgba8,
        pattern::{extract_pattern, find_matches, CoutTrace, NullTrace},
        pixmap::Pixmap,
        topology::Topology,
    };

    fn load_topology(filename: &str) -> Topology {
        let path = format!("test_resources/patterns/{filename}");
        Topology::from_bitmap_path(path).unwrap()
    }

    fn pixmap_with_void_from_path(path: &str) -> Pixmap {
        Pixmap::from_bitmap_path(path)
            .unwrap()
            .without_color(Rgba8::VOID)
    }

    fn assert_extract_inner_outer(name: &str) {
        let folder = "test_resources/extract_pattern";
        let mut pixmap = Pixmap::from_bitmap_path(format!("{folder}/{name}.png")).unwrap();
        let inner = extract_pattern(&mut pixmap);

        // Load expected inner and outer pixmaps
        let expected_inner = pixmap_with_void_from_path(&format!("{folder}/{name}_inner.png"));
        assert_eq!(inner, expected_inner);

        let expected_outer = pixmap_with_void_from_path(&format!("{folder}/{name}_outer.png"));
        assert_eq!(pixmap, expected_outer);
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

    fn assert_pattern_match(pattern_path: &str, world_path: &str, n_solutions: usize) {
        let folder = "test_resources/patterns";
        let pixmap = Pixmap::from_bitmap_path(format!("{folder}/{pattern_path}"))
            .unwrap()
            .without_void_color();
        let pattern = Topology::new(&pixmap);
        let world = Topology::from_bitmap_path(format!("{folder}/{world_path}")).unwrap();

        let trace = NullTrace::new();
        // let trace = CoutTrace::new();

        let matches = find_matches(&world, &pattern, trace);
        assert_eq!(matches.len(), n_solutions);

        // for a_match in &matches {
        //     println!("Match:");
        //
        //     let indent = "  ";
        //
        //     for (seam, phi_seam) in &a_match.seam_map {
        //         println!("{indent}{seam} -> {phi_seam}");
        //     }
        //
        //     for (&region, &phi_region) in &a_match.region_map {
        //         println!(
        //             "{indent}{} -> {}",
        //             pattern[region].color, world[phi_region].color
        //         );
        //     }
        // }
    }

    #[test]
    fn pattern_matches_a() {
        assert_pattern_match("a/pattern.png", "a/match_1.png", 1);
        assert_pattern_match("a/pattern.png", "a/match_2.png", 1);
        assert_pattern_match("a/pattern.png", "a/match_3.png", 1);
        assert_pattern_match("a/pattern.png", "a/match_4.png", 3);
        assert_pattern_match("a/pattern.png", "a/miss_1.png", 0);
    }

    #[test]
    fn pattern_matches_b() {
        assert_pattern_match("b/pattern.png", "b/match_1.png", 1);
        assert_pattern_match("b/pattern.png", "b/match_2.png", 1);
        assert_pattern_match("b/pattern.png", "b/match_3.png", 2);
        assert_pattern_match("b/pattern.png", "b/match_4.png", 4);
        assert_pattern_match("b/pattern.png", "b/miss_1.png", 0);
    }

    #[test]
    fn pattern_matches_c() {
        assert_pattern_match("c/pattern.png", "c/match_1.png", 1);
        assert_pattern_match("c/pattern.png", "c/match_2.png", 2);
        assert_pattern_match("c/pattern.png", "c/miss_1.png", 0);
    }

    #[test]
    fn pattern_matches_single_hole() {
        assert_pattern_match("single_hole/pattern.png", "single_hole/miss_1.png", 0);
        assert_pattern_match("single_hole/pattern.png", "single_hole/match_2.png", 1);
        assert_pattern_match("single_hole/pattern.png", "single_hole/match_3.png", 1);
    }

    #[test]
    fn pattern_matches_two_holes_a() {
        assert_pattern_match("two_holes_a/pattern.png", "two_holes_a/match_1.png", 2);
    }

    #[test]
    fn pattern_matches_two_holes_b() {
        assert_pattern_match("two_holes_b/pattern.png", "two_holes_b/match_1.png", 1);
    }

    #[test]
    fn pattern_matches_two_holes_c() {
        assert_pattern_match("two_holes_c/pattern.png", "two_holes_c/match_1.png", 1);
        assert_pattern_match("two_holes_c/pattern.png", "two_holes_c/miss_1.png", 0);
    }

    #[test]
    fn pattern_matches_gate_a() {
        assert_pattern_match("gate_a/pattern.png", "gate_a/match_1.png", 1);
        assert_pattern_match("gate_a/pattern.png", "gate_a/match_2.png", 1);
        assert_pattern_match("gate_a/pattern.png", "gate_a/match_3.png", 1);
    }
}
