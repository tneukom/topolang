use crate::{
    math::pixel::Vertex,
    topology::{RegionKey, Seam, Topology},
};
use itertools::Itertools;
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
    pub corner_map: BTreeMap<Vertex, Vertex>,
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

impl Index<&Vertex> for Morphism {
    type Output = Vertex;

    fn index(&self, index: &Vertex) -> &Self::Output {
        &self.corner_map[index]
    }
}

impl Morphism {
    pub fn induced_from_seam_map(
        dom: &Topology,
        codom: &Topology,
        seam_map: BTreeMap<Seam, Seam>,
    ) -> Option<Self> {
        let region_map = induced_region_map(dom, codom, &seam_map)?;
        let corner_map = induced_corner_map(&seam_map)?;
        Some(Self {
            region_map,
            corner_map,
            seam_map,
        })
    }

    /// Maps regions to regions of the same color
    pub fn preserves_colors(&self, dom: &Topology, codom: &Topology) -> bool {
        self.region_map
            .iter()
            .all(|(&region, &phi_region)| dom[region].color == codom[phi_region].color)
    }

    /// Check if φ : A → B preserves structure, (neighbor and next_neighbor), i.e.
    ///     start ∘ φ = φ ∘ start
    ///     stop ∘ φ = φ ∘ stop
    ///
    ///     left ∘ φ = φ ∘ left
    ///     right ∘ φ = φ ∘ right
    pub fn preserves_structure(&self, dom: &Topology, codom: &Topology) -> bool {
        self.seam_map.iter().all(|(seam, phi_seam)| {
            self[&dom.left_of(seam)] == codom.left_of(phi_seam)
                && self[&seam.start_corner()] == phi_seam.start_corner()
                && self[&seam.stop_corner()] == phi_seam.stop_corner()
        })
    }

    /// Preserves structure and colors
    pub fn is_homomorphism(&self, dom: &Topology, codom: &Topology) -> bool {
        self.preserves_structure(dom, codom) && self.preserves_colors(dom, codom)
    }

    /// Maps all regions, seams, seam corners of dom
    pub fn is_total(&self, dom: &Topology, include_void_seams: bool) -> bool {
        let seam_total = dom
            .iter_seams()
            .filter(|&seam| include_void_seams || !dom.touches_void(seam))
            .all(|seam| {
                // Seams & corners
                self.seam_map.contains_key(seam)
                    && self.corner_map.contains_key(&seam.start_corner())
                    && self.corner_map.contains_key(&seam.stop_corner())
            });

        if !seam_total {
            return false;
        }

        let regions_total = dom.iter_regions().all(|region| {
            // Regions
            self.region_map.contains_key(&region)
        });
        regions_total
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

/// Does not work for topologies with two regions that are connected by multiple seams, returns an
/// error in that case.
/// Very slow! Could be done faster by iterating over regions, seams around that region while
/// checking if there are duplicate neighbors.
pub fn induced_seam_map(
    dom: &Topology,
    region_phi: &BTreeMap<RegionKey, RegionKey>,
) -> anyhow::Result<Option<BTreeMap<Seam, Seam>>> {
    let mut seam_phi: BTreeMap<Seam, Seam> = BTreeMap::new();

    for (&region_a, &phi_region_a) in region_phi {
        for (&region_b, &phi_region_b) in region_phi {
            let &seam = match dom.seams_between(region_a, region_b).at_most_one() {
                Ok(Some(seam)) => seam,
                Ok(None) => continue,
                Err(_) => anyhow::bail!("Multiple seam between two regions not implemented"),
            };

            let Ok(&phi_seam) = dom.seams_between(phi_region_a, phi_region_b).exactly_one() else {
                return Ok(None);
            };

            if !extend_map(&mut seam_phi, seam, phi_seam) {
                return Ok(None);
            }
        }
    }

    Ok(Some(seam_phi))
}

/// The resulting map satisfies
///     left ∘ φ = φ ∘ left
///     right ∘ φ = φ ∘ right
pub fn induced_region_map(
    dom: &Topology,
    codom: &Topology,
    seam_phi: &BTreeMap<Seam, Seam>,
) -> Option<BTreeMap<RegionKey, RegionKey>> {
    let mut region_phi: BTreeMap<RegionKey, RegionKey> = BTreeMap::new();

    for (seam, phi_seam) in seam_phi {
        // check left side
        let region = dom.left_of(seam);
        let phi_region = codom.left_of(phi_seam);
        if !extend_map(&mut region_phi, region, phi_region) {
            return None;
        }

        // check right side, is this necessary?
        if let (Some(region), Some(phi_region)) = (dom.right_of(seam), codom.right_of(phi_seam)) {
            if !extend_map(&mut region_phi, region, phi_region) {
                return None;
            }
        }
    }

    Some(region_phi)
}

/// The resulting map satisfies
///     start ∘ φ = φ ∘ start
///     stop ∘ φ = φ ∘ stop
pub fn induced_corner_map(seam_phi: &BTreeMap<Seam, Seam>) -> Option<BTreeMap<Vertex, Vertex>> {
    let mut corner_map: BTreeMap<Vertex, Vertex> = BTreeMap::new();

    for (seam, phi_seam) in seam_phi {
        // start corner
        if !extend_map(
            &mut corner_map,
            seam.start_corner(),
            phi_seam.start_corner(),
        ) {
            return None;
        }

        // stop corner
        if !extend_map(&mut corner_map, seam.stop_corner(), phi_seam.stop_corner()) {
            return None;
        }
    }

    Some(corner_map)
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

#[cfg(test)]
mod test {
    use crate::{
        morphism::Morphism,
        topology::{Seam, Topology},
    };
    use anyhow::anyhow;
    use itertools::Itertools;
    use std::collections::BTreeMap;

    /// Seam map determined by left and right region colors.
    fn seam_map_from_colors(
        dom: &Topology,
        codom: &Topology,
    ) -> anyhow::Result<BTreeMap<Seam, Seam>> {
        let mut phi: BTreeMap<Seam, Seam> = BTreeMap::new();

        for seam in dom.iter_seams() {
            let seam_colors = dom.seam_colors(seam);
            let phi_seam = codom
                .iter_seams()
                .filter(|&phi_seam| {
                    let phi_seam_colors = codom.seam_colors(phi_seam);
                    seam_colors == phi_seam_colors
                })
                .exactly_one()
                .map_err(|_| anyhow!("Seam mapping not determined from colors."))?;

            phi.insert(*seam, *phi_seam);
        }

        Ok(phi)
    }

    /// Map each seam of a Topology to itself
    fn trivial_seam_automorphism(topo: &Topology) -> BTreeMap<Seam, Seam> {
        topo.iter_seams().map(|&seam| (seam, seam)).collect()
    }

    #[test]
    fn test_induced_trivial_automorphism() {
        let filenames = ["2a.png", "2b.png", "3a.png", "3b.png", "3c.png", "4a.png"];
        for filename in filenames {
            let path = format!("test_resources/topology/{filename}");
            let topology = Topology::from_bitmap_path(&path).unwrap();
            let seam_phi = trivial_seam_automorphism(&topology);
            let phi = Morphism::induced_from_seam_map(&topology, &topology, seam_phi).unwrap();
            assert!(phi.is_total(&topology, true));
            assert!(phi.is_homomorphism(&topology, &topology));
        }
    }

    fn assert_proper_morphism(dom_filename: &str, codom_filename: &str) {
        let folder = "test_resources/morphism/";
        let dom_path = format!("{folder}/{dom_filename}");
        let codom_path = format!("{folder}/{codom_filename}");
        let dom = Topology::from_bitmap_path(&dom_path).unwrap();
        let codom = Topology::from_bitmap_path(&codom_path).unwrap();

        let seam_phi = seam_map_from_colors(&dom, &codom).unwrap();
        let phi = Morphism::induced_from_seam_map(&dom, &codom, seam_phi).unwrap();

        assert!(phi.is_homomorphism(&dom, &codom));
        assert!(phi.is_total(&dom, false));
    }

    #[test]
    fn morphism_a() {
        assert_proper_morphism("a.png", "phi_a.png");
    }

    #[test]
    fn morphism_b() {
        assert_proper_morphism("b.png", "phi_b.png");
    }

    #[test]
    fn morphism_c() {
        assert_proper_morphism("c.png", "phi_c.png");
    }

    #[test]
    fn morphism_d() {
        assert_proper_morphism("d.png", "phi_d.png");
    }

    #[test]
    fn morphism_e() {
        assert_proper_morphism("e.png", "phi_e.png");
    }
}
