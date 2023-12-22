use crate::{
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

// /// Returns possible candidate seams in `world` that a `seam` in `pattern` could be mapped to.
// /// Returned seam don't have to be atomic.
// pub fn seam_candidates(world: &Topology, pattern: &Topology, seam: &Seam) -> Vec<Seam> {
//     generalized_seams(world)
//         .iter()
//         .filter(|&phi_seam| pattern.left_of(seam) == world.left_of(phi_seam))
//     world.iter_seams().filter()
// }
//
// pub fn search_step(world: &Topology, pattern: &Topology, partial: &mut BTreeMap<Seam, Seam>) {
//     // Find a seam that is not yet assigned
//     let unassigned = pattern.iter_seams().find(|&seam| !partial.contains_key(seam));
//     let Some(unassigned) = unassigned else {
//         // If seams are assigned, check morphism is proper
//         let phi = Morphism::induced_from_seam_map(pattern, world, partial.clone());
//     };
//
//     // Assign the found seam to all possible candidates and recurse
//
// }
//
// pub fn find_matches(world: &Topology, pattern: &Topology) -> Option<Morphism> {
//
// }
