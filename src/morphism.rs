use crate::{
    math::pixel::Corner,
    topology::{RegionKey, Seam, Topology},
};
use std::{
    collections::{btree_map::Entry, BTreeMap},
    ops::Index,
};

// Orientation preserving!
//
// A segment map φ :: Dict{Seam, Seam} is called consistent iff
//
// 1) φ ∘ reversed = reversed ∘ φ
//
// 2) ∀ s, s': left(s) = left(s') => (left ∘ φ)(s) = (left ∘ φ)(s')
//
// 3) ∀ s, s': start(s) = start(s')
struct Morphism {
    // dom: Topology,
    // codom: Topology
    pub region_map: BTreeMap<usize, usize>,
    pub seam_map: BTreeMap<Seam, Seam>,
    pub corner_map: BTreeMap<Corner, Corner>,
}

impl Index<&usize> for Morphism {
    type Output = usize;

    fn index(&self, index: &usize) -> &Self::Output {
        &self.region_map[index]
    }
}

impl Index<&Seam> for Morphism {
    type Output = Seam;

    fn index(&self, index: &Seam) -> &Self::Output {
        &self.seam_map[index]
    }
}

impl Index<&Corner> for Morphism {
    type Output = Corner;

    fn index(&self, index: &Corner) -> &Self::Output {
        &self.corner_map[index]
    }
}

impl Morphism {
    /// Check if φ : A → B preserves structure, (neighbor and next_neighbor), i.e.
    ///     start ∘ φ = φ ∘ start
    ///     stop ∘ φ = φ ∘ stop
    ///
    ///     left ∘ φ = φ ∘ left
    ///     right ∘ φ = φ ∘ right
    pub fn is_homomorphism(&self, dom: &Topology, codom: &Topology) -> bool {
        self.seam_map.iter().all(|(seam, phi_seam)| {
            self[&dom.left_of(seam)] != codom.left_of(phi_seam)
                && self[&seam.start_corner()] != phi_seam.start_corner()
                && self[&seam.stop_corner()] != phi_seam.stop_corner()
        })
    }
}

/// Add (key, value) to the map, if key already exists and map[key] = value returns true
/// otherwise false.
pub fn extend_map<K: Ord, V: Eq>(map: &mut BTreeMap<K, V>, key: K, value: V) -> bool {
    match map.entry(key) {
        Entry::Vacant(vacant) => {
            vacant.insert(value);
            true
        }
        Entry::Occupied(occupied) => occupied.get() == &value,
    }
}

/// The resulting map satisfies
///     left ∘ φ = φ ∘ left
///     right ∘ φ = φ ∘ right
pub fn induced_region_map(
    dom: &Topology,
    codom: &Topology,
    seam_phi: &BTreeMap<Seam, Seam>,
) -> Option<BTreeMap<usize, usize>> {
    let mut comp_phi: BTreeMap<usize, RegionKey> = BTreeMap::new();

    for (seam, phi_seam) in seam_phi {
        // check left side
        let comp = dom.left_of(seam);
        let phi_comp = codom.left_of(phi_seam);
        if !extend_map(&mut comp_phi, comp, phi_comp) {
            return None;
        }

        // check right side, is this necessary?
        let comp = dom.right_of(seam);
        let phi_comp = codom.right_of(phi_seam);
        if !extend_map(&mut comp_phi, comp, phi_comp) {
            return None;
        }
    }

    Some(comp_phi)
}

// /// Induced Morphism from a map of the components.
// pub fn induced_seam_map(
//     dom: &Topology,
//     codom: &Topology,
//     comp_phi: &BTreeMap<usize, usize>,
// ) -> Option<BTreeMap<Seam, Seam>> {
//     let mut seam_phi: BTreeMap<Seam, Seam> = BTreeMap::new();
//
//     for (i_comp, comp) in dom.components.iter().enumerate() {
//         for border in &comp.boundary {
//
//         }
//     }
// }

// function induced_seamφ(dom::Topology, codom::Topology, compmap::Dict{Component,Component})::Dict{Seam,Seam}
//     boundarymap = Dict{Seam,Seam}()
//
//     for comp ∈ dom.components, boundary ∈ comp.boundaries
//         φneighbors = [compmap[rightof(dom, seg)] for seg ∈ boundary.segments]
//         # try out all boundaries of φ
//         φcomp = compmap[comp]
//
//         φsegments = find_boundary(codom, φcomp, φneighbors)
//         if φsegments === nothing
//             return nothing
//         end
//         for (seg, φseg) ∈ zip(boundary.segments, φsegments)
//             boundarymap[seg] = φseg
//         end
//     end
//
//     return boundarymap
// end
