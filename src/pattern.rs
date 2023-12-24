use crate::{
    math::{pixel::Pixel, rgba8::Rgba8},
    morphism::Morphism,
    topology::{Seam, Topology},
};
use std::collections::BTreeMap;

pub fn generalized_seams(topo: &Topology) -> Vec<Seam> {
    let mut seams = Vec::new();
    for border in topo.iter_borders() {
        for i in 0..border.seams.len() {
            for j in 0..border.seams.len() {
                let start = border.seams[i].start;
                let stop = border.seams[(i + j) % border.seams.len()].stop;
                let seam = Seam::new_with_len(start, stop, j + 1);
                seams.push(seam)
            }
        }
    }

    seams
}

/// Returns possible candidate seams in `world` that a `seam` in `pattern` could be mapped to.
/// Returned seam don't have to be atomic.
pub fn seam_candidates(world: &Topology, pattern: &Topology, seam: &Seam) -> Vec<Seam> {
    generalized_seams(world)
        .iter()
        .filter(|&phi_seam| pattern.left_of(seam) == world.left_of(phi_seam))
        .copied()
        .collect()
}

pub fn search_step(
    world: &Topology,
    pattern: &Topology,
    partial: BTreeMap<Seam, Seam>,
    solutions: &mut Vec<Morphism>,
) {
    // Check if partial assignment is consistent
    let Some(phi) = Morphism::induced_from_seam_map(pattern, world, partial.clone()) else {
        return;
    };

    // Find a seam that is not yet assigned
    let unassigned = pattern
        .iter_seams()
        .find(|&seam| !partial.contains_key(seam));
    let Some(unassigned) = unassigned else {
        // If seams are assigned, check morphism is proper
        solutions.push(phi);
        return;
    };

    // Assign the found seam to all possible candidates and recurse
    for phi_unassigned in seam_candidates(world, pattern, unassigned) {
        let mut ext_partial = partial.clone();
        ext_partial.insert(*unassigned, phi_unassigned);
        search_step(world, pattern, ext_partial, solutions)
    }
}

pub fn find_matches(world: &Topology, pattern: &Topology) -> Vec<Morphism> {
    let mut solutions = Vec::new();
    let partial = BTreeMap::new();
    search_step(world, pattern, partial, &mut solutions);
    solutions
}

const PATTERN_FRAME_COLOR: Rgba8 = Rgba8::MAGENTA;

pub fn extract_pattern(pixmap: &mut BTreeMap<Pixel, Rgba8>) -> BTreeMap<Pixel, Rgba8> {
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

    // Extract pixels left of inner_border
    let mut inner = BTreeMap::new();
    for pixel in inner_border.right_pixels() {
        let color = pixmap.remove(&pixel).unwrap();
        inner.insert(pixel, color);
    }

    inner
}

#[cfg(test)]
mod test {
    use crate::{
        math::{pixel::Pixel, rgba8::Rgba8},
        pattern::extract_pattern,
        pixmap::{pixmap_from_path, pixmap_remove_color},
        topology::Topology,
    };
    use std::collections::BTreeMap;

    fn load_topology(filename: &str) -> Topology {
        let path = format!("test_resources/patterns/{filename}");
        Topology::from_bitmap_path(path).unwrap()
    }

    fn pixmap_with_void_from_path(path: &str) -> BTreeMap<Pixel, Rgba8> {
        let mut pixmap = pixmap_from_path(path).unwrap();
        pixmap_remove_color(&mut pixmap, Topology::VOID_COLOR);
        pixmap
    }

    fn assert_extract_inner_outer(name: &str) {
        let folder = "test_resources/extract_pattern";
        let mut pixmap = pixmap_from_path(format!("{folder}/{name}.png")).unwrap();
        let inner = extract_pattern(&mut pixmap);

        // Load expected inner and outer pixmaps
        let expected_inner = pixmap_with_void_from_path(&format!("{folder}/{name}_inner.png"));
        assert_eq!(inner, expected_inner);

        let expected_outer = pixmap_with_void_from_path(&format!("{folder}/{name}_outer.png"));
        assert_eq!(pixmap, expected_outer);
    }

    #[test]
    fn extract_pattern_extract_a() {
        assert_extract_inner_outer("a");
    }

    #[test]
    fn extract_pattern_pattern_b() {
        assert_extract_inner_outer("b");
    }

    #[test]
    fn extract_pattern_pattern_c() {
        assert_extract_inner_outer("c");
    }
}
