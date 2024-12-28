use crate::world::World;
/// TODO: Open questions
/// - Should a Solid automatically include the boundary? For example if a region is fully solid
///   it doesn't make sense to much any superset of it, which would happen if the boundary is not
///   included.
use crate::{
    math::{point::Point, rect::Rect},
    morphism::Morphism,
    pattern::Pattern,
    pixmap::{MaterialMap, Pixmap},
    regions::pixmap_regions,
    topology::Topology,
};
use std::collections::HashMap;

/// A subset of a solid region (connected component of solid pixels).
#[derive(Debug, Clone)]
pub struct SolidArea {
    /// Non-empty
    pub pixels: Vec<Point<i64>>,

    /// Non-empty
    pub bounds: Rect<i64>,
}

impl SolidArea {
    pub fn new(pixels: Vec<Point<i64>>) -> Self {
        assert!(!pixels.is_empty());
        let bounds = Rect::index_bounds(pixels.iter().copied());
        Self { pixels, bounds }
    }
}

/// An area where each pixel is solid and of the same color
#[derive(Debug, Clone)]
pub struct Solid {
    /// Total area of a solid split into subsets of topological regions. A topological region is a
    /// connected area of constant color. A solid area also has constant color but might not be
    /// connected. Consider for example a red circle with green interior and a solid mask cutting
    /// through.
    /// Each element of `areas` is non-empty and `areas` is non-empty.
    /// Maps topological region_id to the solid subset of that area.
    areas: HashMap<usize, SolidArea>,
}

impl Solid {
    /// Create a Solid from a given connected solid area and region_map.
    pub fn new(area: &Vec<Point<i64>>, region_map: &Pixmap<usize>) -> Self {
        assert!(!area.is_empty());

        // Split `area` into subsets of topological regions using `region_map`.
        let mut area_pixels = HashMap::new();
        for &pixel in area {
            let region_id = region_map[pixel];
            area_pixels
                .entry(region_id)
                .or_insert(Vec::new())
                .push(pixel);
        }

        // Create SolidArea from area pixels
        let areas: HashMap<_, _> = area_pixels
            .into_iter()
            .map(|(region_id, area_pixels)| (region_id, SolidArea::new(area_pixels)))
            .collect();

        Self { areas }
    }

    pub fn area(&self) -> impl Iterator<Item = Point<i64>> + Clone + '_ {
        self.areas
            .values()
            .flat_map(|area| area.pixels.iter().copied())
    }
}

pub struct Solids {
    /// Maps pixel to solid index
    pub solid_region_map: Pixmap<usize>,

    /// Non-empty
    pub solids: Vec<Solid>,
}

impl Solids {
    // TODO: Might make more sense to have a PixSet class

    /// Extract the solid area from material_map and split it into connected regions.
    fn solid_regions(material_map: &MaterialMap) -> (Pixmap<usize>, Vec<Vec<Point<i64>>>) {
        // `solid_mask` is defined where the material is solid and otherwise undefined.
        let solid_mask = material_map.filter_map(|_, material| material.is_solid().then_some(()));

        // `solid_map` is a connected component map of `solid_mask`
        let (solid_region_map, covers) = pixmap_regions(&solid_mask);

        let mut solid_regions: Vec<Vec<Point<i64>>> = vec![Vec::new(); covers.len()];
        for (pixel, &region_id) in solid_region_map.iter() {
            solid_regions[region_id].push(pixel);
        }

        (solid_region_map, solid_regions)
    }

    /// `material_region_map` is a connected component map where each component has constant
    /// material (ignoring the solid flag).
    pub fn new(material_map: &MaterialMap, material_region_map: &Pixmap<usize>) -> Self {
        // Connected solid regions
        let (solid_region_map, solid_regions) = Self::solid_regions(material_map);

        // Create a `Solid` from each solid region in `solid_regions`
        let solids: Vec<_> = solid_regions
            .iter()
            .map(|solid_region| Solid::new(solid_region, material_region_map))
            .collect();

        Self {
            solids,
            solid_region_map,
        }
    }
}

/// A subset of rigid transformations (https://en.wikipedia.org/wiki/Rigid_transformation),
/// currently only translations
pub struct SolidTransform {
    pub translation: Point<i64>,
}

impl SolidTransform {
    pub fn new(translation: Point<i64>) -> Self {
        Self { translation }
    }

    /// All translations that map the `lhs` rectangle into the `rhs` rectangle.
    /// Upper bound of returned rectangle is inclusive!
    pub fn translations_fitting_rect_into(lhs: Rect<i64>, rhs: Rect<i64>) -> Rect<i64> {
        Rect::low_high(rhs.low() - lhs.low(), rhs.high() - lhs.size() - lhs.low())
    }

    /// Solid transformation that are compatible with the morphism `phi` on solid.
    pub fn transforms_compatible_with_morphism<'a>(
        solid: &'a Solid,
        dom: &'a Topology,
        codom: &'a Topology,
        phi: &'a Morphism,
    ) -> impl Iterator<Item = Self> + 'a {
        // Compute candidates translations that map `solid` into `codom` such that material regions
        // are preserved.
        // Checking if a candidate transform is compatible with a morphism is expensive so we
        // should try to have as few candidates as possible.
        let area_matches = solid.areas.iter().map(|(&material_region_id, solid_area)| {
            let phi_material_region_id = phi.region_map[&material_region_id];
            let phi_material_region = &codom.regions[&phi_material_region_id];
            Self::translations_fitting_rect_into(solid_area.bounds, phi_material_region.bounds)
        });

        let candidates = area_matches.reduce(Rect::intersect).unwrap();

        candidates.iter_closed().filter_map(|translation| {
            let transform = Self::new(translation);
            // Check if translation is compatible with morphism
            transform
                .is_compatible_with_morphism(solid, dom, codom, phi)
                .then_some(transform)
        })
    }

    /// Is there an instance `phi` that is equal to `self` on `solid`?
    /// Note: The struct `Morphism` does not fully define a morphism between `dom` and `codom`, it
    /// only defines the mapping on seams between regions.
    /// Panics if `dom` is not defined on `solid_region`.
    pub fn is_compatible_with_morphism(
        &self,
        solid: &Solid,
        dom: &Topology,
        codom: &Topology,
        phi: &Morphism,
    ) -> bool {
        // Check if translation is compatible with morphism region map, in other words does the
        // following diagram commute?
        //
        //               self
        // dom_pixel─────────────►codom_pixel
        //      │                     │
        //      │dom.region_map       │codom.region_map
        //      ▼                     ▼
        // dom_region────────────►codom_region
        //               phi
        solid.area().all(|dom_pixel| {
            let codom_pixel = dom_pixel + self.translation;
            let dom_region = dom.region_map[dom_pixel];
            let Some(&codom_region) = codom.region_map.get(codom_pixel) else {
                // codom is not even defined for codom_pixel, not compatible
                return false;
            };

            phi.region_map[&dom_region] == codom_region
        })
    }
}

/// A solid morphism maps each solid region in the pattern to a region in the world of the same
/// material. The map is rigid for each solid.
/// Find all solid morphisms that are compatible with the given topological morphism.
#[inline(never)]
pub fn search_compatible_solid_morphisms(
    pattern: &Pattern,
    world: &Topology,
    phi: &Morphism,
) -> Vec<Vec<SolidTransform>> {
}
pub struct SearchSolidTransforms<'a> {
    pub pattern: &'a Pattern,
    pub world: &'a Topology,
    pub morphism: &'a Morphism,
}

impl<'a> SearchSolidTransforms<'a> {
    pub fn search(&self, on_solution_found: impl FnMut(Vec<SolidTransform>)) {}
}
