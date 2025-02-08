use crate::{
    material::Material,
    morphism::Morphism,
    solver::element::Element,
    topology::{BorderKey, Region, RegionKey, Seam, Topology},
};
use std::fmt::Debug;

pub trait Constraint: Debug {
    fn variables(&self) -> Vec<Element>;

    fn is_satisfied(&self, phi: &Morphism, codom: &Topology) -> bool;
}

/// Assure
///   start_corner * phi = phi * start_corner
///   stop_corner * phi = phi * stop_corner
///   left_region * phi = phi * left_region
///   on_border * phi = phi * on_border
/// TODO: Does it make sense to split this into multiple constraints?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PreservesSeam {
    seam: Seam,

    /// Region left of seam
    left_of_seam_key: RegionKey,

    on_border_key: BorderKey,
}

impl PreservesSeam {
    pub fn new(dom: &Topology, seam: Seam) -> Self {
        Self {
            left_of_seam_key: dom.left_of(seam),
            on_border_key: dom.seam_border(seam),
            seam,
        }
    }
}

impl Constraint for PreservesSeam {
    fn variables(&self) -> Vec<Element> {
        vec![
            self.seam.into(),
            self.seam.start_corner().into(),
            self.seam.stop_corner().into(),
            self.left_of_seam_key.into(),
            self.on_border_key.into(),
        ]
    }

    fn is_satisfied(&self, phi: &Morphism, codom: &Topology) -> bool {
        let phi_seam = phi.seam_map[&self.seam];

        phi.corner_map[&self.seam.start_corner()] == phi_seam.start_corner()
            && phi.corner_map[&self.seam.stop_corner()] == phi_seam.stop_corner()
            && phi.region_map[&self.left_of_seam_key] == codom.left_of(phi_seam)
            && phi.border_map[&self.on_border_key] == codom.seam_border(phi_seam)
    }
}

/// Only applicable if reverse of seam is in domain of phi
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PreservesSeamReverse {
    seam: Seam,
}

impl PreservesSeamReverse {
    pub fn new(seam: Seam) -> Self {
        Self { seam }
    }
}

impl Constraint for PreservesSeamReverse {
    fn variables(&self) -> Vec<Element> {
        vec![self.seam.into(), self.seam.atom_reversed().into()]
    }

    fn is_satisfied(&self, phi: &Morphism, _codom: &Topology) -> bool {
        let phi_seam = phi.seam_map[&self.seam];
        let phi_reverse_seam = phi.seam_map[&self.seam.atom_reversed()];

        phi_seam.atom_reversed() == phi_reverse_seam
    }
}

/// region material must match phi(region) material
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PreservesMaterial {
    region_key: RegionKey,

    material: Material,
}

impl PreservesMaterial {
    pub fn new(region_key: RegionKey, material: Material) -> Self {
        Self {
            region_key,
            material,
        }
    }
}

impl Constraint for PreservesMaterial {
    fn variables(&self) -> Vec<Element> {
        vec![self.region_key.into()]
    }

    fn is_satisfied(&self, phi: &Morphism, codom: &Topology) -> bool {
        let phi_region_key = phi.region_map[&self.region_key];
        let phi_region = &codom[phi_region_key];
        self.material.matches(phi_region.material)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PreservesBorderCount {
    region_key: RegionKey,

    border_count: usize,
}

impl PreservesBorderCount {
    pub fn new(region_key: RegionKey, border_count: usize) -> Self {
        Self {
            region_key,
            border_count,
        }
    }
}

impl Constraint for PreservesBorderCount {
    fn variables(&self) -> Vec<Element> {
        vec![self.region_key.into()]
    }

    fn is_satisfied(&self, phi: &Morphism, codom: &Topology) -> bool {
        let phi_region_key = phi.region_map[&self.region_key];
        let phi_region = &codom[phi_region_key];
        phi_region.boundary.borders.len() == self.border_count
    }
}

/// Unlike the other Constraints this one is expensive to construct because we need to clone Region.
/// The morphism is rigid (https://en.wikipedia.org/wiki/Rigid_transformation) on solid areas.
/// See `Morphism::is_rigid_on_region`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PreservesSolid {
    region_key: RegionKey,

    region: Region,
}

impl PreservesSolid {
    pub fn new(dom: &Topology, region_key: RegionKey) -> Self {
        let region = dom[region_key].clone();
        assert!(region.material.is_solid());
        Self { region_key, region }
    }
}

impl Constraint for PreservesSolid {
    fn variables(&self) -> Vec<Element> {
        let mut variables = vec![self.region_key.into()];
        for seam in self.region.iter_seams() {
            variables.push(seam.into());
        }
        variables
    }

    fn is_satisfied(&self, phi: &Morphism, codom: &Topology) -> bool {
        let phi_region_key = phi.region_map[&self.region_key];
        let phi_region = &codom[phi_region_key];

        phi.is_rigid_on_region(&self.region, &phi_region)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PreservesBorderOrientation {
    border_key: BorderKey,

    is_outer: bool,
}

impl PreservesBorderOrientation {
    pub fn new(border_key: BorderKey, is_outer: bool) -> Self {
        Self {
            border_key,
            is_outer,
        }
    }
}

impl Constraint for PreservesBorderOrientation {
    fn variables(&self) -> Vec<Element> {
        vec![self.border_key.into()]
    }

    fn is_satisfied(&self, phi: &Morphism, codom: &Topology) -> bool {
        let phi_border_key = phi.border_map[&self.border_key];
        let phi_border = &codom[phi_border_key];
        phi_border.is_outer == self.is_outer
    }
}

/// Assure mapped seams are not overlapping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NonOverlappingSeams {
    seam_a: Seam,
    seam_b: Seam,
}

impl NonOverlappingSeams {
    pub fn new(seam_a: Seam, seam_b: Seam) -> Self {
        Self { seam_a, seam_b }
    }
}

impl Constraint for NonOverlappingSeams {
    fn variables(&self) -> Vec<Element> {
        vec![self.seam_a.into(), self.seam_b.into()]
    }

    fn is_satisfied(&self, phi: &Morphism, codom: &Topology) -> bool {
        let phi_seam_a = phi.seam_map[&self.seam_a];
        let phi_seam_b = phi.seam_map[&self.seam_b];
        let overlapping = codom.are_seams_overlapping(phi_seam_a, phi_seam_b);
        !overlapping
    }
}

/// Assure mapped regions are distinct
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DistinctRegions {
    region_a_key: RegionKey,
    region_b_key: RegionKey,
}

impl DistinctRegions {
    pub fn new(region_a_key: RegionKey, region_b_key: RegionKey) -> Self {
        Self {
            region_a_key,
            region_b_key,
        }
    }
}

impl Constraint for DistinctRegions {
    fn variables(&self) -> Vec<Element> {
        vec![self.region_a_key.into(), self.region_b_key.into()]
    }

    fn is_satisfied(&self, phi: &Morphism, _codom: &Topology) -> bool {
        let phi_region_a_key = phi.region_map[&self.region_a_key];
        let phi_region_b_key = phi.region_map[&self.region_b_key];
        phi_region_a_key != phi_region_b_key
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnyConstraint {
    PreservesSeam(PreservesSeam),
    PreservesSeamReverse(PreservesSeamReverse),
    PreservesMaterial(PreservesMaterial),
    PreservesBorderCount(PreservesBorderCount),
    PreservesSolid(PreservesSolid),
    PreservesBorderOrientation(PreservesBorderOrientation),
    NonOverlappingSeams(NonOverlappingSeams),
    DistinctRegions(DistinctRegions),
}

impl AnyConstraint {
    pub fn as_constraint(&self) -> &dyn Constraint {
        match self {
            Self::PreservesSeam(this) => this,
            Self::PreservesSeamReverse(this) => this,
            Self::PreservesMaterial(this) => this,
            Self::PreservesBorderCount(this) => this,
            Self::PreservesSolid(this) => this,
            Self::PreservesBorderOrientation(this) => this,
            Self::NonOverlappingSeams(this) => this,
            Self::DistinctRegions(this) => this,
        }
    }
}

impl Constraint for AnyConstraint {
    fn variables(&self) -> Vec<Element> {
        self.as_constraint().variables()
    }

    fn is_satisfied(&self, phi: &Morphism, codom: &Topology) -> bool {
        self.as_constraint().is_satisfied(phi, codom)
    }
}

/// Creates constraints that enforce a proper Morphism from dom to codom
// - material is preserved
// - solids are preserved
// - seam structure is preserved (start, stop corners, left material)
// - reverse is preserved
// - no spare borders
// - border orientation is preserved
// - seams are not overlapping
// - injective region mapping
pub fn morphism_constraints(dom: &Topology) -> Vec<AnyConstraint> {
    let mut constraints: Vec<AnyConstraint> = Vec::new();

    // Preserve seams & reverse
    for seam in dom.iter_seams() {
        let constraint = PreservesSeam::new(dom, seam);
        constraints.push(AnyConstraint::PreservesSeam(constraint));

        if dom.contains_seam(seam.atom_reversed()) {
            let constraint = PreservesSeamReverse::new(seam);
            constraints.push(AnyConstraint::PreservesSeamReverse(constraint));
        }
    }

    // Preserve region material & solid regions and border count
    for (&region_key, region) in &dom.regions {
        let constraint = PreservesMaterial::new(region_key, region.material);
        constraints.push(AnyConstraint::PreservesMaterial(constraint));

        let constraint = PreservesBorderCount::new(region_key, region.boundary.borders.len());
        constraints.push(AnyConstraint::PreservesBorderCount(constraint));

        if region.material.is_solid() {
            let constraint = PreservesSolid::new(dom, region_key);
            constraints.push(AnyConstraint::PreservesSolid(constraint));
        }
    }

    // Make sure region mapping is injective
    for &region_a_key in dom.iter_region_keys() {
        for &region_b_key in dom.iter_region_keys() {
            if region_a_key < region_b_key {
                let constraint = DistinctRegions::new(region_a_key, region_b_key);
                constraints.push(AnyConstraint::DistinctRegions(constraint));
            }
        }
    }

    // Preserve border orientation (inner/outer borders are mapped to inner/outer borders),
    // seam non overlapping
    for (border_key, border) in dom.iter_borders_with_key() {
        let constraint = PreservesBorderOrientation::new(border_key, border.is_outer);
        constraints.push(AnyConstraint::PreservesBorderOrientation(constraint));

        for seam_a in border.atomic_seams() {
            for seam_b in border.atomic_seams() {
                if seam_a < seam_b {
                    let constraint = NonOverlappingSeams::new(seam_a, seam_b);
                    constraints.push(AnyConstraint::NonOverlappingSeams(constraint));
                }
            }
        }
    }

    constraints
}

pub fn constraints_are_satisfied(
    phi: &Morphism,
    codom: &Topology,
    constraints: &Vec<AnyConstraint>,
) -> bool {
    constraints
        .iter()
        .all(|constraint| constraint.is_satisfied(phi, codom))
}

#[cfg(test)]
mod test {
    use crate::{
        morphism::{test::seam_map_from_colors, Morphism},
        solver::constraints::{constraints_are_satisfied, morphism_constraints},
        topology::Topology,
    };

    fn assert_proper_morphism(dom_filename: &str, codom_filename: &str) {
        let folder = "test_resources/morphism/";
        let dom = Topology::load(format!("{folder}/{dom_filename}")).unwrap();
        let codom = Topology::load(format!("{folder}/{codom_filename}")).unwrap();

        let seam_phi = seam_map_from_colors(&dom, &codom).unwrap();
        let phi = Morphism::induced_from_seam_map(&dom, &codom, seam_phi).unwrap();

        let constraints = morphism_constraints(&dom);
        assert!(constraints_are_satisfied(&phi, &codom, &constraints));
        // for constraint in constraints {
        //     let is_satisfied = constraint.is_satisfied(&phi, &codom);
        //     println!("{constraint:?} is_satisfied: {is_satisfied}");
        // }

        // Single seam mapping defects
        // TODO: Meh, this will fail no seam overlap in all cases, so pretty useless
        for seam in dom.iter_seams() {
            for phi_seam in codom.iter_seams() {
                if phi_seam != phi.seam_map[&seam] {
                    let mut defective_phi = phi.clone();
                    defective_phi.seam_map.insert(seam, phi_seam);
                    assert!(!constraints_are_satisfied(
                        &defective_phi,
                        &codom,
                        &constraints
                    ));
                }
            }
        }
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
