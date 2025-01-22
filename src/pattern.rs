use crate::{
    material::Material,
    math::pixel::Corner,
    morphism::Morphism,
    topology::{BorderKey, RegionKey, Seam, SeamIndex, SeamMaterials, StrongRegionKey, Topology},
};
use itertools::Either;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

/// Returns all seams (including non-atomic ones) in topo
/// Returns all Seams `seam` (including non-atomic ones) where the material on the left of `seam`
/// is a match of `left_material`.
#[inline(never)]
pub fn generalized_seams(topo: &Topology, left_material: Material) -> Vec<Seam> {
    // TODO: Create a function runs() in CycleSegments that returns multiple consecutive segments
    //   joined.
    let mut seams = Vec::new();

    for region in topo.regions.values() {
        if !left_material.matches(region.material) {
            continue;
        }

        for border in &region.boundary.borders {
            for i in 0..border.seams_len() {
                // All seams that go around exactly once are equivalent, so we only include one.
                let len = if i == 0 {
                    // Include seam that goes around the border.
                    border.seams_len()
                } else {
                    // Skip seam that goes around.
                    border.seams_len() - 1
                };

                for j in 0..len {
                    let start = border.atomic_seam(i).start;
                    let stop = border.atomic_seam((i + j) % border.seams_len()).stop;
                    let seam = Seam::new_with_len(start, stop, j + 1);
                    seams.push(seam)
                }
            }
        }
    }

    // Very slow, only for debugging
    // use crate::utils::find_duplicate_by;
    // let duplicate_seam = find_duplicate_by(&seams, |lhs, rhs| topo.seams_equivalent(lhs, rhs));
    // assert!(duplicate_seam.is_none());
    seams
}

/// A seam that has not yet been assigned in a Morphism, its corners and left side might
/// already be assigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnassignedSeam {
    /// Seam in the domain
    seam: Seam,

    /// Region on the left of seam
    region_key: RegionKey,

    /// Region on the right of seam
    right_region_key: Option<RegionKey>,

    /// Index of Border that contains seam
    border_index: usize,

    /// Left and right material of seam
    materials: SeamMaterials,

    /// `phi(self.seam)`
    phi_seam: Option<Seam>,

    /// `phi(self.region_key)`
    phi_region_key: Option<RegionKey>,

    /// phi(self.right_region_key)
    phi_right_region_key: Option<RegionKey>,

    /// `phi(self.border_index)`
    phi_border_index: Option<usize>,

    /// `phi(self.seam.start_corner)`
    phi_start_corner: Option<Corner>,

    /// `phi(self.seam.stop_corner)`
    phi_stop_corner: Option<Corner>,
}

impl UnassignedSeam {
    /// Local morphism at the given seam
    #[inline(never)]
    pub fn local_at(pattern: &Topology, phi: &Morphism, seam: Seam) -> Self {
        let SeamIndex {
            region_key,
            i_border,
            ..
        } = pattern.seam_index(seam).unwrap();
        let border_key = BorderKey::new(region_key, i_border);
        let right_region_key = pattern.right_of(seam);

        Self {
            seam,
            region_key,
            right_region_key,
            border_index: i_border,
            materials: pattern.seam_materials(seam),
            phi_seam: phi.seam_map.get(&seam).copied(),
            phi_region_key: phi.region_map.get(&region_key).copied(),
            phi_right_region_key: right_region_key
                .and_then(|right_region_key| phi.region_map.get(&right_region_key).copied()),
            phi_border_index: phi
                .border_map
                .get(&border_key)
                .map(|phi_border_key| phi_border_key.i_border),
            phi_start_corner: phi.corner_map.get(&seam.start_corner()).copied(),
            phi_stop_corner: phi.corner_map.get(&seam.stop_corner()).copied(),
        }
    }

    /// List of unassigned seams given a pattern and a partial Morphism `phi`
    /// If returned list is empty phi is fully defined
    /// TODO: Rename to something like unassigned_locals
    #[inline(never)]
    pub fn candidates(pattern: &Topology, phi: &Morphism) -> Vec<Self> {
        pattern
            .iter_seams()
            .filter(|seam| !phi.seam_map.contains_key(seam))
            .map(|seam| Self::local_at(pattern, phi, seam))
            .collect()
    }

    /// lhs > rhs means lhs should be assigned before rhs because it is more restricted
    /// by the already assigned seams and/or restricts other items more than rhs.
    /// Not a total order (not antisymmetric)
    fn compare_heuristic(&self, other: &Self) -> Ordering {
        let cmp_tuple = |unassigned: &UnassignedSeam| {
            (
                unassigned.phi_region_key.is_some(),
                unassigned.phi_start_corner.is_some(),
                unassigned.phi_stop_corner.is_some(),
                unassigned.right_region_key.is_some(),
            )
        };

        cmp_tuple(self).cmp(&cmp_tuple(other))
    }

    /// Choose the best unassigned seam to continue the search
    #[inline(never)]
    pub fn choose(pattern: &Topology, phi: &Morphism) -> Option<Self> {
        Self::candidates(pattern, phi)
            .into_iter()
            .max_by(Self::compare_heuristic)
    }

    // #[inline(never)]
    // pub fn is_consistent(&self, world: &Topology, pattern: &Topology) -> bool {
    //     let phi_seam = self.phi_seam.unwrap();
    //
    //     // Check if left side materials match
    //     let left_material = pattern[pattern.left_of(self.seam)].material;
    //     let phi_left_material = world[world.left_of(phi_seam)].material;
    //     if !left_material.matches(phi_left_material) {
    //         // Inconsistent left side color
    //         return false;
    //     }
    //
    //     if self.right_region_key.is_some() {
    //         // TODO: Is this necessary if phi_right_region_key is not None
    //         if !phi_seam.is_atom() {
    //             return false;
    //         }
    //
    //         if self.phi_right_region_key.is_none() {
    //             return false;
    //         }
    //
    //         let Some(right_of_phi_seam) = world.right_of(phi_seam) else {
    //             // phi_seam is not reversible in world
    //             return false;
    //         };
    //
    //         let right_material = pattern[pattern.right_of(self.seam).unwrap()].material;
    //         let phi_right_material = world[right_of_phi_seam].material;
    //         if !right_material.matches(phi_right_material) {
    //             // Inconsistent right color
    //             return false;
    //         }
    //     }
    //
    //     if let Some(phi_left) = self.phi_region_key {
    //         if phi_left != world.left_of(phi_seam) {
    //             // inconsistent left side region
    //             return false;
    //         }
    //     }
    //
    //     if let Some(phi_start_corner) = self.phi_start_corner {
    //         if phi_start_corner != phi_seam.start_corner() {
    //             // inconsistent start corner
    //             return false;
    //         }
    //     }
    //
    //     if let Some(phi_stop_corner) = self.phi_stop_corner {
    //         if phi_stop_corner != phi_seam.stop_corner() {
    //             // inconsistent stop corner
    //             return false;
    //         }
    //     }
    //
    //     true
    // }

    /// Given a partial `self` find fully defined candidates.
    #[inline(never)]
    pub fn assignment_candidates_impl(
        self,
        pattern: &Topology,
        world: &Topology,
        candidates: &mut Vec<Self>,
    ) {
        // Region left of seam
        let Some(phi_region_key) = self.phi_region_key else {
            // Assign phi_region_key to possible candidates and recurse
            let region = &pattern.regions[&self.region_key];

            for (&phi_region_key, phi_region) in &world.regions {
                // region and phi_region must have the same number of borders
                if region.boundary.borders.len() != phi_region.boundary.borders.len() {
                    continue;
                }

                // `region` material must match `phi_region.material`
                if !region.material.matches(phi_region.material) {
                    continue;
                }

                let candidate = Self {
                    phi_region_key: Some(phi_region_key),
                    ..self
                };
                candidate.assignment_candidates_impl(pattern, world, candidates);
            }
            return;
        };
        let phi_region = &world.regions[&phi_region_key];

        let Some(phi_border_key) = self.phi_border_index else {
            // Assign phi_border to possible candidates and recurse, the first border in a boundary
            // is the outer border and can only be mapped to the outer border of phi(region_key).
            if self.border_index == 0 {
                let candidate = Self {
                    phi_border_index: Some(0),
                    ..self
                };
                candidate.assignment_candidates_impl(pattern, world, candidates);
            } else {
                let region = &pattern.regions[&self.region_key];
                assert_eq!(
                    region.boundary.borders.len(),
                    phi_region.boundary.borders.len()
                );
                for phi_border_key in 1..phi_region.boundary.borders.len() {
                    let candidate = Self {
                        phi_border_index: Some(phi_border_key),
                        ..self
                    };
                    candidate.assignment_candidates_impl(pattern, world, candidates);
                }
            }
            return;
        };
        let phi_border = &phi_region.boundary.borders[phi_border_key];

        // Separate case for loop seams in the pattern
        if self.seam.is_loop() {
            let phi_start_corner = self.phi_start_corner.unwrap_or(phi_border.corner(0));

            // No guarantee that phi_border is compatible with phi_start_corner
            let Some(phi_seam) = phi_border.loop_seam_with_corner(phi_start_corner) else {
                return;
            };

            let candidate = Self {
                phi_seam: Some(phi_seam),
                phi_start_corner: Some(phi_start_corner),
                phi_stop_corner: Some(phi_start_corner),
                ..self
            };
            candidates.push(candidate);
            return;
        }

        let phi_seam_candidates = if self.right_region_key.is_some() {
            Either::Left(phi_border.atomic_seams())
        } else {
            Either::Right(phi_border.non_loop_seams())
        };

        for phi_seam in phi_seam_candidates {
            // Discard if start or stop corner does match
            if let Some(phi_start_corner) = self.phi_start_corner {
                if phi_start_corner != phi_seam.start_corner() {
                    continue;
                }
            }
            if let Some(phi_stop_corner) = self.phi_stop_corner {
                if phi_stop_corner != phi_seam.stop_corner() {
                    continue;
                }
            }

            // Discard if right side material doesn't match
            // TODO: If right side is mapped also check if that matches
            // let right_phi_seam = world.right_of(phi_seam).unwrap();
            // if let Some(phi_right_of_seam) = self.phi_right_of_seam

            let candidate = Self {
                phi_seam: Some(phi_seam),
                phi_start_corner: Some(phi_seam.start_corner()),
                phi_stop_corner: Some(phi_seam.stop_corner()),
                ..self
            };
            candidates.push(candidate);
        }
    }

    // /// Iterate over assignment candidates for a pattern seam which has a region on both sides
    // /// (left and right).
    // #[inline(never)]
    // fn both_sided_assignment_candidates(&self, world: &Topology, pattern: &Topology) -> Vec<Seam> {
    //     world
    //         .regions
    //         .values()
    //         .filter(|world_region| self.materials.left.matches(world_region.material))
    //         .flat_map(|world_region| world_region.iter_seams())
    //         .filter(|&phi_seam| self.is_possible_assignment(world, pattern, phi_seam))
    //         .collect()
    // }
    //
    // /// Assignment candidates for a pattern seam which has a region only on the left side.
    // #[inline(never)]
    // fn left_sided_assignment_candidates(&self, world: &Topology, pattern: &Topology) -> Vec<Seam> {
    //     let candidates = generalized_seams(world, self.materials.left);
    //
    //     candidates
    //         .into_iter()
    //         .filter(|&phi_seam| self.is_possible_assignment(world, pattern, phi_seam))
    //         .collect()
    // }

    /// Returns possible candidate seams in `world` that `self.seam` could be mapped to.
    /// Returned seam don't have to be atomic.
    #[inline(never)]
    pub fn assignment_candidates(&self, world: &Topology, pattern: &Topology) -> Vec<Seam> {
        let mut candidates = Vec::new();
        self.assignment_candidates_impl(pattern, world, &mut candidates);

        candidates
            .iter()
            .map(|candidate| {
                // assert!(candidate.is_consistent(world, pattern));
                candidate.phi_seam.unwrap()
            })
            .collect()
    }
}

pub struct SearchMorphism<'a> {
    pub world: &'a Topology,
    pub pattern: &'a Topology,
    pub hidden: Option<&'a BTreeSet<StrongRegionKey>>,
}

impl<'a> SearchMorphism<'a> {
    pub fn new(world: &'a Topology, pattern: &'a Topology) -> Self {
        Self {
            world,
            pattern,
            hidden: None,
        }
    }

    #[inline(never)]
    pub fn search_step(
        &self,
        partial: BTreeMap<Seam, Seam>,
        trace: impl Trace,
        on_solution_found: &mut impl FnMut(Morphism),
    ) {
        // Check if partial assignment is consistent
        let Some(phi) = Morphism::induced_from_seam_map(self.pattern, self.world, partial.clone())
        else {
            trace.failed("Partial assignment inconsistent");
            return;
        };

        if !phi.is_homomorphism(self.pattern, self.world) || !phi.is_injective() {
            trace.failed("Morphism inconsistent");
            return;
        }

        if let Some(hidden) = self.hidden {
            // Mapping to hidden regions is not allowed
            for (_, &phi_region) in &phi.region_map {
                let phi_region_strong_key = self.world[phi_region].arbitrary_interior_pixel();
                if hidden.contains(&phi_region_strong_key) {
                    trace.failed("Matched isolated Region");
                    return;
                }
            }
        }

        // Find a seam that is not yet assigned
        let unassigned = UnassignedSeam::choose(self.pattern, &phi);
        let Some(unassigned) = unassigned else {
            // If seams are assigned, check morphism is proper
            trace.success();
            on_solution_found(phi);
            return;
        };

        let assignment_candidates = unassigned.assignment_candidates(self.world, self.pattern);
        if assignment_candidates.is_empty() {
            trace.failed(&format!("No assignment candidates for {unassigned:?}"));
            return;
        }

        // Assign the found seam to all possible candidates and recurse
        for phi_unassigned in assignment_candidates {
            let mut ext_partial = partial.clone();
            ext_partial.insert(unassigned.seam, phi_unassigned);
            trace.assign(unassigned.seam, phi_unassigned);
            self.search_step(ext_partial, trace.recurse(), on_solution_found);
        }
    }

    #[inline(never)]
    pub fn find_matches(&self, trace: impl Trace) -> Vec<Morphism> {
        let mut solutions = Vec::new();
        let partial = BTreeMap::new();
        self.search_step(partial, trace, &mut |solution| solutions.push(solution));
        solutions
    }

    #[inline(never)]
    pub fn find_first_match(&self, trace: impl Trace) -> Option<Morphism> {
        let phis = self.find_matches(trace);
        phis.into_iter().next()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CoutTrace {
    level: usize,
}

impl CoutTrace {
    pub fn new() -> Self {
        Self { level: 0 }
    }

    pub fn assign(&self, seam: Seam, phi_seam: Seam) {
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
    fn assign(&self, seam: Seam, phi_seam: Seam);

    fn failed(&self, cause: &str);

    fn success(&self);

    fn recurse(&self) -> Self;
}

impl Trace for CoutTrace {
    fn assign(&self, seam: Seam, phi_seam: Seam) {
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
    fn assign(&self, _seam: Seam, _phi_seam: Seam) {}

    fn failed(&self, _cause: &str) {}

    fn success(&self) {}

    fn recurse(&self) -> Self {
        *self
    }
}

#[cfg(test)]
mod test {
    use crate::{
        field::RgbaField,
        material::Material,
        math::rgba8::Rgba8,
        pattern::{NullTrace, SearchMorphism},
        pixmap::MaterialMap,
        topology::Topology,
        utils::IntoT,
    };
    use std::path::Path;

    const PATTERN_FRAME_MATERIAL: Material = Material::from_rgba(Rgba8::MAGENTA);

    #[inline(never)]
    pub fn extract_pattern(material_map: &mut MaterialMap) -> MaterialMap {
        let topo = Topology::new(material_map.clone());

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

    fn pixmap_with_void_from_path(path: impl AsRef<Path>) -> MaterialMap {
        RgbaField::load(path)
            .unwrap()
            .intot::<MaterialMap>()
            .without(Material::VOID)
    }

    fn assert_extract_inner_outer(name: &str) {
        let folder = "test_resources/extract_pattern";
        let mut pixmap = RgbaField::load(format!("{folder}/{name}.png"))
            .unwrap()
            .into();
        let inner = extract_pattern(&mut pixmap);

        // Load expected inner and outer pixmaps
        let expected_inner = pixmap_with_void_from_path(format!("{folder}/{name}_inner.png"));
        assert!(inner.defined_equals(&expected_inner));

        let expected_outer = pixmap_with_void_from_path(format!("{folder}/{name}_outer.png"));
        // pixmap.save(format!("{folder}/{name}_outer_actual.png")).unwrap();
        assert!(pixmap.defined_equals(&expected_outer));
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
        let pattern_material_map = RgbaField::load(format!("{folder}/{pattern_path}"))
            .unwrap()
            .intot::<MaterialMap>();
        let pattern = Topology::new(pattern_material_map).filter_by_material(Material::is_not_void);

        let world_material_map = RgbaField::load(format!("{folder}/{world_path}"))
            .unwrap()
            .into();
        let world = Topology::new(world_material_map);

        let trace = NullTrace::new();
        // let trace = CoutTrace::new();

        let matches = SearchMorphism::new(&world, &pattern).find_matches(trace);
        for phi in &matches {
            println!("Morphism:");
            for (seam, phi_seam) in &phi.seam_map {
                println!("  {seam} -> {phi_seam}");
            }
        }

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
    fn fail() {
        assert_pattern_match("c/pattern.png", "c/match_1.png", 1);
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
        assert_pattern_match("c/pattern.png", "c/miss_1.png", 0);
        assert_pattern_match("c/pattern.png", "c/miss_2.png", 0);
    }

    #[test]
    fn pattern_matches_single_hole() {
        assert_pattern_match("single_hole/pattern.png", "single_hole/miss_1.png", 0);
        assert_pattern_match("single_hole/pattern.png", "single_hole/match_2.png", 1);
        assert_pattern_match("single_hole/pattern.png", "single_hole/match_3.png", 1);
    }

    #[test]
    fn pattern_matches_interior_hole() {
        assert_pattern_match("interior_hole/pattern.png", "interior_hole/miss_1.png", 0);
        assert_pattern_match("interior_hole/pattern.png", "interior_hole/miss_2.png", 0);
        assert_pattern_match("interior_hole/pattern.png", "interior_hole/match_1.png", 1);
        assert_pattern_match("interior_hole/pattern.png", "interior_hole/match_2.png", 1);
        assert_pattern_match("interior_hole/pattern.png", "interior_hole/match_3.png", 1);
        assert_pattern_match("interior_hole/pattern.png", "interior_hole/match_4.png", 1);
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

    #[test]
    fn pattern_matches_rule_frame_a() {
        assert_pattern_match("rule_frame_a/pattern.png", "rule_frame_a/match_1.png", 1);
        assert_pattern_match("rule_frame_a/pattern.png", "rule_frame_a/match_2.png", 1);
        assert_pattern_match("rule_frame_a/pattern.png", "rule_frame_a/match_3.png", 1);
        assert_pattern_match("rule_frame_a/pattern.png", "rule_frame_a/match_4.png", 1);
    }

    #[test]
    fn pattern_matches_rule_frame_b() {
        assert_pattern_match("rule_frame_b/pattern.png", "rule_frame_b/match_1.png", 1);
        assert_pattern_match("rule_frame_b/pattern.png", "rule_frame_b/match_2.png", 1);
        assert_pattern_match("rule_frame_b/pattern.png", "rule_frame_b/match_3.png", 1);
        assert_pattern_match("rule_frame_b/pattern.png", "rule_frame_b/match_4.png", 1);
    }

    #[test]
    fn pattern_matches_solid_a() {
        assert_pattern_match("solid/a/pattern.png", "solid/a/match_1.png", 1);
        assert_pattern_match("solid/a/pattern.png", "solid/a/miss_1.png", 0);
        assert_pattern_match("solid/a/pattern.png", "solid/a/miss_2.png", 0);
        assert_pattern_match("solid/a/pattern.png", "solid/a/miss_3.png", 0);
        assert_pattern_match("solid/a/pattern.png", "solid/a/miss_4.png", 0);
    }

    #[test]
    fn pattern_matches_solid_b() {
        assert_pattern_match("solid/b/pattern.png", "solid/b/match_1.png", 1);
        assert_pattern_match("solid/b/pattern.png", "solid/b/miss_1.png", 0);
    }

    #[test]
    fn pattern_matches_solid_c() {
        assert_pattern_match("solid/c/pattern.png", "solid/c/match_1.png", 1);
        assert_pattern_match("solid/c/pattern.png", "solid/c/miss_1.png", 0);
    }

    #[test]
    fn wildcard_a() {
        assert_pattern_match("wildcard_a/pattern.png", "wildcard_a/match_1.png", 1);
        assert_pattern_match("wildcard_a/pattern.png", "wildcard_a/match_2.png", 1);
        assert_pattern_match("wildcard_a/pattern.png", "wildcard_a/miss_1.png", 0);
        assert_pattern_match("wildcard_a/pattern.png", "wildcard_a/miss_2.png", 0);
        assert_pattern_match("wildcard_a/pattern.png", "wildcard_a/miss_3.png", 0);
        assert_pattern_match("wildcard_a/pattern.png", "wildcard_a/miss_4.png", 0);
    }

    #[test]
    fn wildcard_b() {
        assert_pattern_match("wildcard_b/pattern.png", "wildcard_b/match_1.png", 1);
        assert_pattern_match("wildcard_b/pattern.png", "wildcard_b/miss_1.png", 0);
        assert_pattern_match("wildcard_b/pattern.png", "wildcard_b/miss_2.png", 0);
    }

    #[test]
    fn wildcard_c() {
        assert_pattern_match("wildcard_c/pattern.png", "wildcard_c/match_1.png", 1);
        assert_pattern_match("wildcard_c/pattern.png", "wildcard_c/match_2.png", 1);
        assert_pattern_match("wildcard_c/pattern.png", "wildcard_c/match_3.png", 2);
        // Not 100% sure this is the right answer
        assert_pattern_match("wildcard_c/pattern.png", "wildcard_c/match_4.png", 2);

        assert_pattern_match("wildcard_c/pattern.png", "wildcard_c/miss_1.png", 0);
        assert_pattern_match("wildcard_c/pattern.png", "wildcard_c/miss_2.png", 0);
    }
}
