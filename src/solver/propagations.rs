use crate::{math::pixel::Corner, solver::constraints::Variables};
/// Do we need the following Propagations?
/// - Derive border from seam, not needed if we guess border before seam
/// - Derive right side region of seam, not needed since we can derive the left side of the  reverse
///   of the Seam.
/// TODO:
/// - Derive outer border from region
/// - Derive inner border if only one not assigned
use crate::{
    morphism::Morphism,
    solver::element::Element,
    topology::{BorderKey, RegionKey, Seam, SeamCorner, Topology},
};
use ahash::HashSet;
use itertools::Itertools;
use std::{fmt::Debug, hash::Hash};

pub trait Propagation: Debug + Variables {
    fn derives(&self) -> Element;

    fn derive(&self, phi: &Morphism, codom: &Topology) -> anyhow::Result<Element>;

    fn name(&self) -> &'static str;
}

/// Given a border and a corner on it, derive the seam with a region on both sides, starting or
/// stopping at the given corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BothSidedSeamFromCorner {
    /// phi(border) already defined
    border_key: BorderKey,

    corner: SeamCorner,

    /// phi(seam.corner(corner_name)) already defined
    seam: Seam,

    variables: [Element; 2],
}

impl BothSidedSeamFromCorner {
    pub fn new(border_key: BorderKey, corner: SeamCorner, seam: Seam) -> Self {
        Self {
            border_key,
            corner,
            seam,
            variables: [border_key.into(), seam.corner(corner).into()],
        }
    }
}

impl Variables for BothSidedSeamFromCorner {
    fn variables(&self) -> &[Element] {
        &self.variables
    }
}

impl Propagation for BothSidedSeamFromCorner {
    fn derives(&self) -> Element {
        self.seam.into()
    }

    #[inline(never)]
    fn derive(&self, phi: &Morphism, codom: &Topology) -> anyhow::Result<Element> {
        // Find phi_seam in codom
        // on phi(border)
        // with phi_seam.corner(self.corner_name) = phi(seam.corner(self.corner_name))
        let phi_border_key = phi.border_map[&self.border_key];
        let phi_border = &codom[phi_border_key];
        let corner = self.seam.corner(self.corner);
        let phi_corner = phi.corner_map[&corner];

        let Some(phi_seam) = phi_border
            .atomic_seams()
            .find(|seam| seam.corner(self.corner) == phi_corner)
        else {
            // It can happen that phi(seam) is incompatible with phi(border(seam))
            anyhow::bail!("No seam found on border with corner");
        };

        Ok(phi_seam.into())
    }

    fn name(&self) -> &'static str {
        "BothSidedSeamFromCorner"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoleSeamOnBorder {
    /// phi(border_key) already defined
    border_key: BorderKey,

    /// border only contains a single seam
    seam: Seam,

    variables: [Element; 1],
}

impl SoleSeamOnBorder {
    pub fn new(border_key: BorderKey, seam: Seam) -> Self {
        Self {
            border_key,
            seam,
            variables: [border_key.into()],
        }
    }
}

impl Variables for SoleSeamOnBorder {
    fn variables(&self) -> &[Element] {
        &self.variables
    }
}

impl Propagation for SoleSeamOnBorder {
    fn derives(&self) -> Element {
        self.seam.into()
    }

    #[inline(never)]
    fn derive(&self, phi: &Morphism, codom: &Topology) -> anyhow::Result<Element> {
        let phi_border_key = phi.border_map[&self.border_key];
        let phi_border = &codom[phi_border_key];
        let phi_seam = phi_border.loop_seam();
        Ok(phi_seam.into())
    }

    fn name(&self) -> &'static str {
        "SoleSeamOnBorder"
    }
}

/// Given start and stop corners of a Seam, derive the Seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SeamFromBothCorners {
    /// phi(border_key) already defined
    border_key: BorderKey,

    seam: Seam,

    variables: [Element; 3],
}

impl SeamFromBothCorners {
    pub fn new(border_key: BorderKey, seam: Seam) -> Self {
        Self {
            border_key,
            seam,
            variables: [
                seam.start_corner().into(),
                seam.stop_corner().into(),
                border_key.into(),
            ],
        }
    }
}

impl Variables for SeamFromBothCorners {
    fn variables(&self) -> &[Element] {
        &self.variables
    }
}

impl Propagation for SeamFromBothCorners {
    fn derives(&self) -> Element {
        self.seam.into()
    }

    #[inline(never)]
    fn derive(&self, phi: &Morphism, codom: &Topology) -> anyhow::Result<Element> {
        let phi_start_corner = phi.corner_map[&self.seam.start_corner()];
        let phi_stop_corner = phi.corner_map[&self.seam.stop_corner()];
        let phi_border_key = phi.border_map[&self.border_key];
        let phi_border = &codom[phi_border_key];
        let Some(phi_seam) = phi_border.seam_between_corners(phi_start_corner, phi_stop_corner)
        else {
            anyhow::bail!("seam_between_corners failed");
        };

        Ok(phi_seam.into())
    }

    fn name(&self) -> &'static str {
        "SeamFromBothCorners"
    }
}

/// Given an atomic Seam derive its reverse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SeamReverse {
    /// phi(seam) is already defined
    seam: Seam,

    variables: [Element; 1],
}

impl SeamReverse {
    pub fn new(seam: Seam) -> Self {
        Self {
            seam,
            variables: [seam.into()],
        }
    }
}

impl Variables for SeamReverse {
    fn variables(&self) -> &[Element] {
        &self.variables
    }
}

impl Propagation for SeamReverse {
    fn derives(&self) -> Element {
        self.seam.atom_reversed().into()
    }

    #[inline(never)]
    fn derive(&self, phi: &Morphism, codom: &Topology) -> anyhow::Result<Element> {
        let phi_seam = phi.seam_map[&self.seam];
        if !phi_seam.is_atom() {
            anyhow::bail!("phi_seam should be atomic.");
        }
        let phi_reversed_seam = phi_seam.atom_reversed();
        if !codom.contains_seam(phi_reversed_seam) {
            // println!("phi_seam: {phi_seam:?}, phi_reversed_seam: {phi_reversed_seam:?}");
            anyhow::bail!("codom does not contain reverse");
        }

        Ok(phi_reversed_seam.into())
    }

    fn name(&self) -> &'static str {
        "SeamReverse"
    }
}

/// Given a Seam derive the Region on its left side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LeftRegionOfSeam {
    /// phi(seam) is already defined
    seam: Seam,

    region_key: RegionKey,

    variables: [Element; 1],
}

impl LeftRegionOfSeam {
    pub fn new(seam: Seam, region_key: RegionKey) -> Self {
        Self {
            seam,
            region_key,
            variables: [seam.into()],
        }
    }
}

impl Variables for LeftRegionOfSeam {
    fn variables(&self) -> &[Element] {
        &self.variables
    }
}

impl Propagation for LeftRegionOfSeam {
    fn derives(&self) -> Element {
        self.region_key.into()
    }

    #[inline(never)]
    fn derive(&self, phi: &Morphism, codom: &Topology) -> anyhow::Result<Element> {
        let phi_seam = phi.seam_map[&self.seam];
        let phi_region_key = codom.left_of(phi_seam);

        Ok(phi_region_key.into())
    }

    fn name(&self) -> &'static str {
        "LeftRegionOfSeam"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CornerFromSeam {
    /// phi(seam) is already defined
    seam: Seam,

    corner: SeamCorner,

    variables: [Element; 1],
}

impl CornerFromSeam {
    pub fn new(seam: Seam, corner: SeamCorner) -> Self {
        Self {
            seam,
            corner,
            variables: [seam.into()],
        }
    }
}

impl Variables for CornerFromSeam {
    fn variables(&self) -> &[Element] {
        &self.variables
    }
}

impl Propagation for CornerFromSeam {
    fn derives(&self) -> Element {
        self.seam.corner(self.corner).into()
    }

    #[inline(never)]
    fn derive(&self, phi: &Morphism, _codom: &Topology) -> anyhow::Result<Element> {
        let phi_seam = phi.seam_map[&self.seam];
        let phi_corner = phi_seam.corner(self.corner);

        Ok(phi_corner.into())
    }

    fn name(&self) -> &'static str {
        "CornerFromSeam"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OuterBorder {
    /// phi(region_key) is already defined
    region_key: RegionKey,

    variables: [Element; 1],
}

impl OuterBorder {
    pub fn new(region_key: RegionKey) -> Self {
        Self {
            region_key,
            variables: [region_key.into()],
        }
    }

    fn outer_border(&self) -> BorderKey {
        BorderKey::new(self.region_key, 0)
    }
}

impl Variables for OuterBorder {
    fn variables(&self) -> &[Element] {
        &self.variables
    }
}

impl Propagation for OuterBorder {
    fn derives(&self) -> Element {
        self.outer_border().into()
    }

    #[inline(never)]
    fn derive(&self, phi: &Morphism, _codom: &Topology) -> anyhow::Result<Element> {
        let &phi_region_key = &phi.region_map[&self.region_key];
        let phi_outer_border = BorderKey::new(phi_region_key, 0);

        Ok(phi_outer_border.into())
    }

    fn name(&self) -> &'static str {
        "OuterBorder"
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LastInnerBorder {
    /// phi(region_key) is already defined
    region_key: RegionKey,

    borders_len: usize,

    inner_border_index: usize,

    variables: Vec<Element>,
}

impl LastInnerBorder {
    pub fn new(region_key: RegionKey, borders_len: usize, inner_border_index: usize) -> Self {
        let mut variables: Vec<_> =
            Self::other_inner_border_keys(region_key, borders_len, inner_border_index)
                .map(|border_key| border_key.into())
                .collect();
        variables.push(region_key.into());
        Self {
            region_key,
            borders_len,
            inner_border_index,
            variables,
        }
    }

    fn inner_border_key(&self) -> BorderKey {
        BorderKey::new(self.region_key, self.inner_border_index)
    }

    fn other_inner_border_keys(
        region_key: RegionKey,
        borders_len: usize,
        inner_border_index: usize,
    ) -> impl Iterator<Item = BorderKey> {
        (1..borders_len).filter_map(move |i_border| {
            (i_border != inner_border_index).then_some(BorderKey::new(region_key, i_border))
        })
    }
}

impl Variables for LastInnerBorder {
    fn variables(&self) -> &[Element] {
        &self.variables
    }
}

impl Propagation for LastInnerBorder {
    fn derives(&self) -> Element {
        self.inner_border_key().into()
    }

    #[inline(never)]
    fn derive(&self, phi: &Morphism, codom: &Topology) -> anyhow::Result<Element> {
        // Find which border in codom is not yet assigned
        let phi_region_key = phi.region_map[&self.region_key];

        let phi_region = &codom[phi_region_key];
        if phi_region.boundary.borders.len() != self.borders_len {
            anyhow::bail!("region border count does not match phi(region) border count.");
        }

        let mut available_phi_inner_border_keys: HashSet<_> = (1..self.borders_len)
            .into_iter()
            .map(|i| BorderKey::new(phi_region_key, i))
            .collect();

        for other_inner_border_key in Self::other_inner_border_keys(
            self.region_key,
            self.borders_len,
            self.inner_border_index,
        ) {
            let phi_other_inner_border_key = phi.border_map[&other_inner_border_key];
            available_phi_inner_border_keys.remove(&phi_other_inner_border_key);
        }

        let Ok(phi_inner_border_key) = available_phi_inner_border_keys.into_iter().exactly_one()
        else {
            anyhow::bail!("Multiple border mapped to the phi_border!");
        };

        Ok(phi_inner_border_key.into())
    }

    fn name(&self) -> &'static str {
        "LastInnerBorder"
    }
}

#[inline(never)]
pub fn morphism_propagations(dom: &Topology) -> Vec<AnyPropagation> {
    let _tracy_span = tracy_client::span!("morphism_propagations");

    let mut propagations: Vec<AnyPropagation> = Vec::new();

    for (&region_key, region) in &dom.regions {
        // OuterBorder
        let outer_border = OuterBorder::new(region_key);
        propagations.push(AnyPropagation::OuterBorder(outer_border));

        // LastInnerBorder
        for inner_border_index in 1..region.boundary.borders.len() {
            let last_inner_border = LastInnerBorder::new(
                region_key,
                region.boundary.borders.len(),
                inner_border_index,
            );
            propagations.push(AnyPropagation::LastInnerBorder(last_inner_border));
        }

        for (i_border, border) in region.boundary.borders.iter().enumerate() {
            let border_key = BorderKey::new(region_key, i_border);

            if border.seams_len() == 1 {
                let seam = border.atomic_seam(0);
                let sole_seam_on_border = SoleSeamOnBorder::new(border_key, seam);
                propagations.push(AnyPropagation::SoleSeamOnBorder(sole_seam_on_border));
            }

            for seam in border.atomic_seams() {
                // SeamFromBothCorners
                let seam_from_both_corners = SeamFromBothCorners::new(border_key, seam);
                propagations.push(AnyPropagation::SeamFromBothCorners(seam_from_both_corners));

                if dom.contains_seam(seam.atom_reversed()) {
                    // BothSidedSeamFromCorner
                    for seam_corner in SeamCorner::ALL {
                        let seam_from_corner =
                            BothSidedSeamFromCorner::new(border_key, seam_corner, seam);
                        propagations
                            .push(AnyPropagation::BothSidedSeamFromCorner(seam_from_corner));
                    }
                }
            }
        }

        for seam in region.iter_seams() {
            // CornerFromSeam
            for corner in SeamCorner::ALL {
                let corner_from_seam = CornerFromSeam::new(seam, corner);
                propagations.push(AnyPropagation::CornerFromSeam(corner_from_seam));
            }

            // LeftRegionOfSeam
            let left_region_of_seam = LeftRegionOfSeam::new(seam, region_key);
            propagations.push(AnyPropagation::LeftRegionOfSeam(left_region_of_seam));

            // SeamReverse
            if dom.contains_seam(seam.atom_reversed()) {
                let seam_reverse = SeamReverse::new(seam);
                propagations.push(AnyPropagation::SeamReverse(seam_reverse));
            }
        }
    }

    propagations
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnyPropagation {
    BothSidedSeamFromCorner(BothSidedSeamFromCorner),
    SeamFromBothCorners(SeamFromBothCorners),
    OuterBorder(OuterBorder),
    LastInnerBorder(LastInnerBorder),
    CornerFromSeam(CornerFromSeam),
    LeftRegionOfSeam(LeftRegionOfSeam),
    SeamReverse(SeamReverse),
    SoleSeamOnBorder(SoleSeamOnBorder),
}

impl AnyPropagation {
    pub fn as_propagation(&self) -> &dyn Propagation {
        match self {
            Self::BothSidedSeamFromCorner(this) => this,
            Self::SeamFromBothCorners(this) => this,
            Self::OuterBorder(this) => this,
            Self::LastInnerBorder(this) => this,
            Self::CornerFromSeam(this) => this,
            Self::LeftRegionOfSeam(this) => this,
            Self::SeamReverse(this) => this,
            Self::SoleSeamOnBorder(this) => this,
        }
    }
}

impl Variables for AnyPropagation {
    fn variables(&self) -> &[Element] {
        self.as_propagation().variables()
    }
}

impl Propagation for AnyPropagation {
    fn derives(&self) -> Element {
        self.as_propagation().derives()
    }

    fn derive(&self, phi: &Morphism, codom: &Topology) -> anyhow::Result<Element> {
        self.as_propagation().derive(phi, codom)
    }

    fn name(&self) -> &'static str {
        self.as_propagation().name()
    }
}
