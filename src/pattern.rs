use crate::{
    math::rgba8::Rgba8,
    morphism::Morphism,
    pixmap::Pixmap,
    topology::{Seam, Topology},
    utils::find_duplicate_by,
};
use std::collections::BTreeMap;

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

/// Returns possible candidate seams in `world` that a `seam` in `pattern` could be mapped to.
/// Returned seam don't have to be atomic.
pub fn seam_candidates(world: &Topology, pattern: &Topology, seam: &Seam) -> Vec<Seam> {
    generalized_seams(world)
        .iter()
        .filter(|&phi_seam| {
            pattern[pattern.left_of(seam)].color == world[world.left_of(phi_seam)].color
        })
        .copied()
        .collect()
}

#[derive(Clone, Copy)]
pub struct Trace {
    level: usize,
}

impl Trace {
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

pub fn search_step(
    world: &Topology,
    pattern: &Topology,
    partial: BTreeMap<Seam, Seam>,
    solutions: &mut Vec<Morphism>,
    trace: Trace,
) {
    // Check if partial assignment is consistent
    let Some(phi) = Morphism::induced_from_seam_map(pattern, world, partial.clone()) else {
        trace.failed();
        return;
    };

    // Find a seam that is not yet assigned
    let unassigned = pattern
        .iter_seams()
        .find(|&seam| !partial.contains_key(seam));
    let Some(unassigned) = unassigned else {
        // If seams are assigned, check morphism is proper
        trace.success();
        solutions.push(phi);
        return;
    };

    // Assign the found seam to all possible candidates and recurse
    for phi_unassigned in seam_candidates(world, pattern, unassigned) {
        let mut ext_partial = partial.clone();
        ext_partial.insert(*unassigned, phi_unassigned);
        trace.assign(unassigned, &phi_unassigned);
        search_step(world, pattern, ext_partial, solutions, trace.recurse())
    }
}

pub fn find_matches(world: &Topology, pattern: &Topology) -> Vec<Morphism> {
    let mut solutions = Vec::new();
    let partial = BTreeMap::new();
    let trace = Trace::new();
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

    // TODO: Use frame.boundary.indices() when -> impl Trait is stable
    let i_inner_border = (0..frame.boundary.len())
        .find(|&i| i != frame.outer_border)
        .unwrap();
    let inner_border = &frame.boundary[i_inner_border];

    pixmap.extract_right(inner_border)
}

#[cfg(test)]
mod test {
    use crate::{
        math::rgba8::Rgba8,
        pattern::{extract_pattern, find_matches},
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

    #[test]
    fn match_pattern_a() {
        let folder = "test_resources/patterns";
        let pixmap = Pixmap::from_bitmap_path(format!("{folder}/pattern_a.png"))
            .unwrap()
            .without_void_color();
        let pattern = Topology::new(&pixmap);
        let world = Topology::from_bitmap_path(format!("{folder}/match_a2.png")).unwrap();

        let matches = find_matches(&world, &pattern);
        assert_eq!(matches.len(), 1);
    }
}
