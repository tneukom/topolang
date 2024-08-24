use crate::{
    math::pixel::Corner,
    topology::{BorderKey, RegionKey, Seam, Topology},
};
use itertools::Itertools;
use std::{
    collections::{btree_map::Entry, BTreeMap, BTreeSet},
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
pub struct Morphism {
    // dom: Topology,
    // codom: Topology
    pub region_map: BTreeMap<RegionKey, RegionKey>,
    pub seam_map: BTreeMap<Seam, Seam>,
    pub corner_map: BTreeMap<Corner, Corner>,
    pub border_map: BTreeMap<BorderKey, BorderKey>,
}

impl Index<RegionKey> for Morphism {
    type Output = RegionKey;

    fn index(&self, index: RegionKey) -> &Self::Output {
        &self.region_map[&index]
    }
}

impl Index<Seam> for Morphism {
    type Output = Seam;

    fn index(&self, index: Seam) -> &Self::Output {
        &self.seam_map[&index]
    }
}

impl Index<Corner> for Morphism {
    type Output = Corner;

    fn index(&self, index: Corner) -> &Self::Output {
        &self.corner_map[&index]
    }
}

impl Index<BorderKey> for Morphism {
    type Output = BorderKey;

    fn index(&self, index: BorderKey) -> &Self::Output {
        &self.border_map[&index]
    }
}

pub fn is_injective<T: Ord>(phi: &BTreeMap<T, T>) -> bool {
    let mut image = BTreeSet::new();
    for value in phi.values() {
        if !image.insert(value) {
            return false;
        }
    }

    true
}

impl Morphism {
    #[inline(never)]
    pub fn induced_from_seam_map(
        dom: &Topology,
        codom: &Topology,
        seam_map: BTreeMap<Seam, Seam>,
    ) -> Option<Self> {
        let region_map = induced_region_map(dom, codom, &seam_map)?;
        let corner_map = induced_corner_map(&seam_map)?;
        let border_map = induced_border_map(dom, codom, &seam_map)?;
        Some(Self {
            region_map,
            corner_map,
            seam_map,
            border_map,
        })
    }

    pub fn is_injective(&self) -> bool {
        is_injective(&self.region_map) && is_injective(&self.border_map)
    }

    /// Maps regions to regions of the same material
    #[inline(never)]
    pub fn preserves_materials(&self, dom: &Topology, codom: &Topology) -> bool {
        self.region_map.iter().all(|(&region, &phi_region)| {
            let dom_material = dom[region].material;
            let codom_material = codom[phi_region].material;
            if dom_material.is_solid() {
                // match rigid rgb or normal rgb
                (codom_material.is_normal() || codom_material.is_solid())
                    && dom_material.rgb() == codom_material.rgb()
            } else {
                // exact color match otherwise
                dom[region].material == codom[phi_region].material
            }
        })
    }

    // fn region_is_subset_of(lhs_region: RegionKey, lhs_topo: &Topology, right_topo: &Topology) {
    //     lhs_topo.iter_region_interior(lhs_region)
    //
    // }

    // /// Returns true if the left side is equal to the right side plus an offset.
    // pub fn is_translation_of(
    //     lhs: RegionKey,
    //     lhs_topo: &Topology,
    //     rhs: RegionKey,
    //     rhs_topo: &Topology,
    // ) -> bool {
    //     let offset =
    //     let offset = lhs_topo[lhs].bounding_rect().low() - rhs_topo[rhs].bounding_rect().low();
    //
    //     let lhs_offset_interior = lhs_topo
    //         .iter_region_interior(lhs)
    //         .map(|pixel| pixel + offset);
    //
    //     let rhs_interior = rhs_topo.iter_region_interior(rhs);
    //
    //     // Assumes iteration order is same independent of translations.
    //     Iterator::eq(lhs_offset_interior, rhs_interior)
    // }

    /// Rigid regions are mapped to regions with exactly the same shape
    #[inline(never)]
    pub fn preserves_rigidity(&self, dom: &Topology, codom: &Topology) -> bool {
        self.region_map
            .iter()
            .all(|(&region_key, &phi_region_key)| {
                let region = &dom[region_key];
                let phi_region = &codom[phi_region_key];
                if region.material.is_solid() {
                    // If region is rigid, phi_region must be a translation of region
                    region.boundary.is_translation_of(&phi_region.boundary)
                } else {
                    true
                }
            })
    }

    /// Check if phi : A → B preserves structure, (neighbor and next_neighbor), i.e.
    ///   start * phi = phi * start
    ///   stop * phi = phi * stop
    ///
    ///   left * phi = phi * left
    ///   phi * reversed = reversed * phi
    #[inline(never)]
    pub fn preserves_structure(&self, dom: &Topology, codom: &Topology) -> bool {
        for (seam, phi_seam) in &self.seam_map {
            if self[seam.start_corner()] != phi_seam.start_corner() {
                return false;
            }

            if self[seam.stop_corner()] != phi_seam.stop_corner() {
                return false;
            }

            if self[dom.left_of(seam)] != codom.left_of(phi_seam) {
                return false;
            }

            if let Some(&phi_reversed) = self.seam_map.get(&seam.reversed()) {
                if !phi_seam.is_atom() {
                    // phi(seam^-1) = phi(seam)^-1 means phi(seam) must be an atom, otherwise it is not reversible.
                    return false;
                }

                if phi_reversed != phi_seam.reversed() {
                    return false;
                }
            }
        }

        true
    }

    /// Map inner borders to inner borders and outer borders to outer borders.
    #[inline(never)]
    pub fn preserves_inner_outer(&self, dom: &Topology, codom: &Topology) -> bool {
        self.border_map
            .iter()
            .all(|(&border, &phi_border)| dom[border].is_outer == codom[phi_border].is_outer)
    }

    #[inline(never)]
    pub fn non_overlapping(&self, dom: &Topology, codom: &Topology) -> bool {
        self.border_map.iter().all(|(&border, &phi_border)| {
            let len_phi_seams: usize = dom[border]
                .seams
                .iter()
                .filter_map(|seam| {
                    let phi_seam = self.seam_map.get(seam)?;
                    Some(phi_seam.len)
                })
                .sum();

            len_phi_seams <= codom[phi_border].seams.len()
        })
    }

    /// Preserves structure and materials
    #[inline(never)]
    pub fn is_homomorphism(&self, dom: &Topology, codom: &Topology) -> bool {
        self.preserves_structure(dom, codom)
            && self.preserves_materials(dom, codom)
            && self.non_overlapping(dom, codom)
            && self.preserves_inner_outer(dom, codom)
    }

    /// Maps all regions, seams, seam corners of dom
    #[inline(never)]
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

        let regions_total = dom.iter_region_keys().all(|region| {
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
///   left ∘ φ = φ ∘ left
///   right ∘ φ = φ ∘ right
#[inline(never)]
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
    }

    Some(region_phi)
}

#[inline(never)]
pub fn induced_border_map(
    dom: &Topology,
    codom: &Topology,
    seam_phi: &BTreeMap<Seam, Seam>,
) -> Option<BTreeMap<BorderKey, BorderKey>> {
    let mut border_phi: BTreeMap<BorderKey, BorderKey> = BTreeMap::new();

    for (seam, phi_seam) in seam_phi {
        let border = dom.seam_border(seam);
        let phi_border = codom.seam_border(phi_seam);
        if !extend_map(&mut border_phi, border, phi_border) {
            return None;
        }
    }

    Some(border_phi)
}

/// The resulting map satisfies
///   start ∘ φ = φ ∘ start
///   stop ∘ φ = φ ∘ stop
#[inline(never)]
pub fn induced_corner_map(seam_phi: &BTreeMap<Seam, Seam>) -> Option<BTreeMap<Corner, Corner>> {
    let mut corner_map: BTreeMap<Corner, Corner> = BTreeMap::new();

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

#[cfg(test)]
mod test {
    use crate::{
        field::RgbaField,
        morphism::Morphism,
        pixmap::MaterialMap,
        topology::{Seam, Topology},
        utils::IntoT,
    };
    use anyhow::anyhow;
    use itertools::Itertools;
    use std::collections::BTreeMap;

    /// Seam map determined by left and right region materials.
    fn seam_map_from_colors(
        dom: &Topology,
        codom: &Topology,
    ) -> anyhow::Result<BTreeMap<Seam, Seam>> {
        let mut phi: BTreeMap<Seam, Seam> = BTreeMap::new();

        for seam in dom.iter_seams() {
            let seam_colors = dom.seam_materials(seam);
            let phi_seam = codom
                .iter_seams()
                .filter(|&phi_seam| {
                    let phi_seam_colors = codom.seam_materials(phi_seam);
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
            let topology = RgbaField::load(&path)
                .unwrap()
                .intot::<MaterialMap>()
                .into();
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
        let dom = RgbaField::load(&dom_path)
            .unwrap()
            .intot::<MaterialMap>()
            .into();
        let codom = RgbaField::load(&codom_path)
            .unwrap()
            .intot::<MaterialMap>()
            .into();

        let seam_phi = seam_map_from_colors(&dom, &codom).unwrap();
        let phi = Morphism::induced_from_seam_map(&dom, &codom, seam_phi).unwrap();

        assert!(phi.is_homomorphism(&dom, &codom));
        assert!(phi.is_total(&dom, true));
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
