use crate::math::rect::RectBounds;
/// Unionfind crates
/// https://crates.io/crates/union-find
/// https://crates.io/crates/disjoint-sets
///
/// https://crates.io/crates/partitions
/// Has clear function
///
/// https://docs.rs/petgraph/latest/src/petgraph/unionfind.rs.html#16-27
use crate::{
    field::Field,
    math::pixel::{Side, SideName},
    union_find::UnionFind,
};
use ahash::{HashMap, HashMapExt};
use itertools::Itertools;

use crate::{
    math::{pixel::Pixel, rect::Rect},
    pixmap::Pixmap,
};

pub struct CompactLabels {
    remap: Vec<usize>,
    counter: usize,
}

/// Maps the given labels to [0,...,n] where n is the number of distinct labels.
impl CompactLabels {
    #[inline(never)]
    pub fn new(max_label: usize) -> Self {
        Self {
            remap: vec![usize::MAX; max_label],
            counter: 0,
        }
    }

    #[inline(never)]
    pub fn clear(&mut self) {
        self.counter = 0;
        for i in 0..self.remap.len() {
            self.remap[i] = usize::MAX;
        }
    }

    pub fn remap(&mut self, label: usize) -> usize {
        if self.remap[label] == usize::MAX {
            self.remap[label] = self.counter;
            self.counter += 1;
        }
        self.remap[label]
    }

    #[inline(never)]
    pub fn compact<'a>(&mut self, labels: impl IntoIterator<Item = &'a mut usize>) -> usize {
        for label in labels {
            *label = self.remap(*label);
        }
        self.counter
    }

    #[inline(never)]
    pub fn compact_masked<'a>(
        &mut self,
        labels: impl IntoIterator<Item = &'a mut Option<usize>>,
    ) -> usize {
        for label in labels {
            if let Some(label) = label {
                *label = self.remap(*label);
            }
        }
        self.counter
    }
}

trait NeighborsFn<T: Copy> {
    fn call<const N: usize>(&mut self, field: &Field<T>, center: usize, neighbors: [usize; N]);

    fn for_each(&mut self, field: &Field<T>) {
        let width = field.width() as usize;
        let height = field.height() as usize;

        // Top pixels
        for x in 1..width {
            let center = x;
            self.call(field, center, [center - 1]);
        }

        // Left pixels
        for y in 1..height {
            let center = y * width;
            self.call(field, center, [center - width])
        }

        // Interior pixels
        for y in 1..height {
            for x in 1..width {
                let center = y * width + x;
                self.call(
                    field,
                    center,
                    [center - 1, center - width, center - width - 1],
                );
            }
        }
    }
}

struct BuildRegionsUnionFind {
    union_find: UnionFind,
}

impl<T: Copy + Eq> NeighborsFn<T> for BuildRegionsUnionFind {
    // Build UnionFind data structure for equivalent regions
    fn call<const N: usize>(&mut self, field: &Field<T>, center: usize, neighbors: [usize; N]) {
        let center_value = field.as_slice()[center];
        for neighbor in neighbors {
            let neighbor_value = field.as_slice()[neighbor];
            if neighbor_value == center_value {
                self.union_find.union(center, neighbor);
            }
        }
    }
}

#[inline(never)]
pub fn field_regions_fast<T: Copy + Eq>(field: &Field<T>) -> Field<usize> {
    let union_find = UnionFind::new(field.len());
    let mut build_union_find = BuildRegionsUnionFind { union_find };
    build_union_find.for_each(field);
    let labeling = build_union_find.union_find.into_roots();
    Field::from_linear(field.bounds(), labeling)
}

/// Returns connected components of `field`
#[inline(never)]
pub fn field_regions<T: Copy + Eq>(field: &Field<T>) -> Field<usize> {
    // TODO: Reuse UnionFind so we don't allocate each time
    let mut union_find = UnionFind::new(field.len());

    for (center, &color) in field.enumerate() {
        for side in [SideName::TopLeft, SideName::Top, SideName::Left] {
            let neighbor = center.neighbor(side);
            if let Some(neighbor_color) = field.get(neighbor).copied() {
                if color == neighbor_color {
                    union_find.union(
                        field.linear_index(center).unwrap(),
                        field.linear_index(neighbor).unwrap(),
                    );
                }
            }
        }
    }

    let labeling = union_find.into_roots();
    Field::from_linear(field.bounds(), labeling)
}

#[inline(never)]
pub fn mask_field<T: Copy, S>(field: &Field<T>, mask: &Field<Option<S>>) -> Field<Option<T>> {
    let elems = field
        .iter()
        .zip(mask.iter())
        .map(|(el, mask_el)| mask_el.as_ref().map(|_| el.clone()))
        .collect();
    Field::from_linear(field.bounds(), elems)
}

#[inline(never)]
pub fn pixmap_regions<T: Copy + Eq>(pixmap: &Pixmap<T>) -> (Pixmap<usize>, Vec<Rect<i64>>) {
    let mut region_field = field_regions_fast(&pixmap.field);

    // TODO: field_regions_fast should directly ignore Nones
    // Remove regions from None
    let mut region_field = mask_field(&mut region_field, &pixmap.field);

    // TODO: Reuse CompactLabels so we don't allocate each time
    let mut compact_labels = CompactLabels::new(region_field.len());
    compact_labels.compact_masked(region_field.iter_mut());

    // Compute bounding rect for each region
    let n_regions = compact_labels.counter;
    let mut region_bounding_rects = vec![Rect::EMPTY; n_regions];
    for (pixel, &region_id) in region_field.enumerate() {
        if let Some(region_id) = region_id {
            let region_bounding_rect = &mut region_bounding_rects[region_id];
            *region_bounding_rect = region_bounding_rect.bounds_with(pixel);
        }
    }

    // Bounding box upper bound is exclusive
    // TODO: Ugly
    for rect in &mut region_bounding_rects {
        *rect = rect.inc_high();
    }

    (Pixmap::new(region_field), region_bounding_rects)
}

/// Mapping each side to the region_id on its right side
pub type RegionBoundary = HashMap<Side, Option<usize>>;

pub fn iter_pixmap_sides<T: Copy>(
    pixmap: &Pixmap<T>,
) -> impl Iterator<Item = (Side, T, Option<T>)> + '_ {
    pixmap.iter().flat_map(|(pixel, left)| {
        pixel.sides_ccw().map(|side| {
            let right = pixmap.get(side.right_pixel());
            (side, left, right)
        })
    })
}

#[inline(never)]
pub fn region_boundaries(region_map: &Pixmap<usize>, n_regions: usize) -> Vec<RegionBoundary> {
    let mut boundaries = vec![RegionBoundary::new(); n_regions];

    for (side, left, right) in iter_pixmap_sides(region_map) {
        let boundary = &mut boundaries[left];
        if Some(left) != right {
            boundary.insert(side, right);
        }
    }

    boundaries
}

/// Split a set of sides into cycles. Each cycle starts with its smallest side. The list of cycles
/// is ordered by the first side in each cycle. This means the first cycle is the outer cycle.
#[inline(never)]
pub fn split_boundary_into_cycles<T>(mut sides: HashMap<Side, T>) -> Vec<Vec<(Side, T)>> {
    let mut cycles = Vec::new();

    // Pop the next side after `side` from `sides` and return it.
    fn pop_next_side_on_boundar<T>(sides: &mut HashMap<Side, T>, side: Side) -> Option<(Side, T)> {
        for next_side in side.continuing_sides() {
            // There is always exactly one continuing side
            if let Some(next_color) = sides.remove(&next_side) {
                return Some((next_side, next_color));
            }
        }

        None
    }

    while !sides.is_empty() {
        // Pop first element
        let mut side = *sides.keys().next().unwrap();
        let color = sides.remove(&side).unwrap();

        // Extract cycle
        let mut cycle = vec![(side, color)];
        while let Some((next_side, next_color)) = pop_next_side_on_boundar(&mut sides, side) {
            cycle.push((next_side, next_color));
            side = next_side;
        }

        // Make sure cycle starts with the smallest element
        let i_min = cycle.iter().position_min_by_key(|(side, _)| side).unwrap();
        cycle.rotate_left(i_min);

        cycles.push(cycle);
    }

    // Sort cycles by first element
    cycles.sort_by_key(|cycle| cycle[0].0);
    cycles
}

/// Area to the left of boundary, see topology_terms.md
pub fn area_left_of_boundary(boundary: impl Iterator<Item = Side>) -> Vec<Pixel> {
    // TODO: Could this return an Iterator<Item = Pixel>? Would avoid one allocation
    // Collect rows of pixels between left and right borders.
    let mut left_right_sides: Vec<_> = boundary
        .filter(|side| side.name == SideName::Left || side.name == SideName::Right)
        .collect();

    assert!(!left_right_sides.is_empty());

    left_right_sides.sort_by_key(|side| side.left_pixel);

    let mut area = Vec::new();
    for (left_side, right_side) in left_right_sides.into_iter().tuples() {
        assert_eq!(left_side.left_pixel.y, right_side.left_pixel.y);
        for x in left_side.left_pixel.x..=right_side.left_pixel.x {
            let pixel = Pixel::new(x, left_side.left_pixel.y);
            area.push(pixel);
        }
    }

    area
}

/// Bounding rectangle of the area left of boundary, see `area_left_of_boundary`.
pub fn area_left_of_boundary_bounds(boundary: impl Iterator<Item = Side>) -> Rect<i64> {
    RectBounds::iter_bounds(boundary.map(|side| side.left_pixel)).inc_high()
}

/// See `area_left_of_boundary`
pub fn area_right_of_boundary(boundary: impl Iterator<Item = Side>) -> Vec<Pixel> where
{
    area_left_of_boundary(boundary.map(Side::reversed))
}

/// See `area_left_of_boundary_bounds`
pub fn area_right_of_boundary_bounds(boundary: impl Iterator<Item = Side>) -> Rect<i64> {
    area_left_of_boundary_bounds(boundary.map(Side::reversed))
}

#[cfg(test)]
mod test {
    use crate::{
        field::{Field, RgbaField},
        math::{generic::EuclidDivRem, pixel::Side, point::Point, rect::Rect, rgba8::Rgba8},
        pixmap::{iter_sides_in_rect, MaterialMap, Pixmap, RgbaMap},
        regions::{
            area_left_of_boundary, area_right_of_boundary, pixmap_regions, region_boundaries,
            split_boundary_into_cycles,
        },
        utils::IntoT,
    };
    use ahash::{HashMap, HashMapExt};
    use itertools::Itertools;
    use std::collections::{BTreeMap, HashSet};

    fn generate_field(bounds: Rect<i64>) -> Field<Rgba8> {
        Field::from_map(bounds, |index| {
            let red = index.x.euclid_rem(256) as u8;
            let green = index.y.euclid_rem(256) as u8;
            Rgba8::new(red, green, 255, 255)
        })
    }

    /// Collect the boundary of the area with the given value, in other words the sides that have
    /// a pixel of the given value on the left side and a different value on the right side.
    fn boundary_of<T: Ord + Copy>(map: &Pixmap<T>, interior: T) -> HashMap<Side, Option<T>> {
        let mut boundary = HashMap::new();
        for side in iter_sides_in_rect(map.bounding_rect()) {
            if map.get(side.left_pixel) == Some(interior) {
                let right_color = map.get(side.right_pixel());
                if right_color != Some(interior) {
                    boundary.insert(side, right_color);
                }
            }
        }
        boundary
    }

    /// Compute region map from color map, given that there are no two regions with the same
    /// color.
    fn pixmap_region_given_distinct_colors(color_map: &Pixmap<Rgba8>) -> Pixmap<usize> {
        let mut color_to_region = BTreeMap::new();
        let mut region_map = Pixmap::nones(color_map.bounding_rect());
        for (pixel, color) in color_map.iter() {
            let len = color_to_region.len();
            let region_id = color_to_region.entry(color).or_insert(len);
            region_map.set(pixel, *region_id);
        }
        region_map
    }

    /// Returns true if two region maps are equivalent, meaning there exists a map phi
    /// that such that phi(lhs[pixel]) = rhs[pixel] for each pixel.
    fn region_map_equiv(lhs: &Pixmap<usize>, rhs: &Pixmap<usize>) -> bool {
        let mut phi = BTreeMap::new();
        for (lhs_region_id, rhs_region_id) in lhs.values().zip_eq(rhs.values()) {
            let phi_lhs_region_id = phi.entry(lhs_region_id).or_insert(rhs_region_id);
            if *phi_lhs_region_id != rhs_region_id {
                return false;
            }
        }
        return true;
    }

    fn test_pixmap_regions(filename: &str) {
        let folder = "test_resources/regions/region_map";
        let color_map = Field::load(format!("{folder}/{filename}"))
            .unwrap()
            .intot::<RgbaMap>();

        let (region_map, area_covers) = pixmap_regions(&color_map);

        let n_regions = area_covers.len();
        for region_id in region_map.values() {
            assert!(region_id < n_regions, "region_ids should be 0,...,n");
        }

        // let region_field = region_map.to_field(255);
        // let region_field_rgba = region_field.map(|id| Rgba8::new(*id as u8, 0, 0, 255));
        // region_field_rgba
        //     .save(format!("{folder}/out_{filename}"))
        //     .unwrap();

        let expected_region_map = pixmap_region_given_distinct_colors(&color_map);

        assert!(region_map_equiv(&region_map, &expected_region_map));
    }

    #[test]
    fn pixmap_regions_a() {
        test_pixmap_regions("a.png");
    }

    // TODO: Is this still required now that tiled Pixmap is gone?
    #[test]
    fn regions_multiple_tiles() {
        test_pixmap_regions("multiple_tiles_a.png");
        test_pixmap_regions("multiple_tiles_b.png");
        test_pixmap_regions("multiple_tiles_c.png");
    }

    fn test_region_boundaries(filename: &str) {
        let folder = "test_resources/regions/region_boundary";
        let color_map = Field::load(format!("{folder}/{filename}"))
            .unwrap()
            .intot::<MaterialMap>();

        // Create region_map and compute boundaries of each region
        let (region_map, area_covers) = pixmap_regions(&color_map);
        let boundaries = region_boundaries(&region_map, area_covers.len());

        // Compare each region boundary with reference implementation
        for (region_id, boundary) in boundaries.iter().enumerate() {
            let expected_boundary = boundary_of(&region_map, region_id);
            assert_eq!(boundary, &expected_boundary);
        }
    }

    #[test]
    fn region_boundaries_a() {
        test_region_boundaries("a.png");
    }

    // TODO: Is this still required now that tiled Pixmap is gone?
    #[test]
    fn region_boundaries_multiple_tiles() {
        test_region_boundaries("multiple_tiles_a.png");
        test_region_boundaries("multiple_tiles_b.png");
        test_region_boundaries("multiple_tiles_c.png");
    }

    fn area_boundary(area: &HashSet<Point<i64>>) -> HashSet<Side> {
        area.iter()
            // iter over all sides of each pixel in area
            .flat_map(|pixel| pixel.sides_ccw())
            // keep only sides with a different region on the right side
            .filter(|side| !area.contains(&side.right_pixel()))
            .collect()
    }

    /// Helper function to split boundary into cycles on HashSet<Side> instead of HashMap<Side, T>
    /// Naturally mapping HashSet<Side> to HashMap<Side, ()> and Vec<(Side, ()> to Vec<Side>
    fn split_set_boundary_into_cycles(boundary: HashSet<Side>) -> Vec<Vec<Side>> {
        let side_map: HashMap<Side, ()> = boundary.into_iter().map(|side| (side, ())).collect();
        let cycles = split_boundary_into_cycles(side_map);
        cycles
            .into_iter()
            .map(|cycle| cycle.into_iter().map(|(side, _)| side).collect())
            .collect()
    }

    fn test_left_of_border(area: HashSet<Point<i64>>, n_sides: usize) {
        let boundary = area_boundary(&area);
        assert_eq!(boundary.len(), n_sides);

        let borders = split_set_boundary_into_cycles(boundary.into());
        assert_eq!(borders.len(), 1);

        let border = &borders[0];
        let left_of_border = area_left_of_boundary(border.iter().copied());
        assert_eq!(area, left_of_border.into_iter().collect());
    }

    /// The cycle from a border and the reverse of the border should start at the same corner.
    /// See also regarding this: min_side_cw_ccw test in pixel.rs
    #[test]
    fn compatible_reverse_borders() {
        let sides_ccw: HashSet<_> = Point(0, 0).sides_ccw().into_iter().collect();
        let cycles_ccw = split_set_boundary_into_cycles(sides_ccw);
        assert_eq!(cycles_ccw.len(), 1);

        let sides_cw: HashSet<_> = Point(0, 0).sides_cw().into_iter().collect();
        let cycles_cw = split_set_boundary_into_cycles(sides_cw);
        assert_eq!(cycles_cw.len(), 1);

        assert_eq!(
            cycles_cw[0][0].start_corner(),
            cycles_ccw[0][0].start_corner()
        );
    }

    #[test]
    fn simple_left_of_border_a() {
        let area: HashSet<_> = [Point(0, 0)].into();
        test_left_of_border(area, 6);
    }

    #[test]
    fn simple_left_of_border_b() {
        let area: HashSet<_> = [Point(0, 0), Point(1, 0)].into();
        test_left_of_border(area, 10);
    }

    #[test]
    fn simple_left_of_border_c() {
        let area: HashSet<_> = [Point(0, 0), Point(1, 0), Point(0, 1), Point(1, 1)].into();
        test_left_of_border(area, 14);
    }

    /// Set of pixels with the given color
    fn pixmap_color_area<T: Eq + Copy>(pixmap: &Pixmap<T>, area_color: T) -> HashSet<Point<i64>> {
        pixmap
            .iter()
            .filter(|&(_, pixel_color)| pixel_color == area_color)
            .map(|kv| kv.0)
            .collect()
    }

    fn test_right_of_border(filename: &str) {
        let folder = "test_resources/regions/right_of_border";
        let path = format!("{folder}/{filename}");
        let color_map: RgbaMap = RgbaField::load(path).unwrap().into();
        let red_area = pixmap_color_area(&color_map, Rgba8::RED);
        let blue_area = pixmap_color_area(&color_map, Rgba8::BLUE);

        let red_boundary = area_boundary(&red_area);
        let red_borders = split_set_boundary_into_cycles(red_boundary.into());
        assert_eq!(red_borders.len(), 2); // inner and outer border

        let inner_border = &red_borders[1];
        let right_of_inner_border = area_right_of_boundary(inner_border.iter().copied());
        let right_of_inner_border_set: HashSet<_> = right_of_inner_border.into_iter().collect();
        assert_eq!(right_of_inner_border_set, blue_area);
    }

    #[test]
    fn right_of_border_1() {
        test_right_of_border("1.png")
    }

    #[test]
    fn right_of_border_2() {
        test_right_of_border("2.png")
    }

    #[test]
    fn right_of_border_3() {
        test_right_of_border("3.png")
    }

    #[test]
    fn right_of_border_4() {
        test_right_of_border("4.png")
    }

    fn assert_proper_components(filename: &str, count: usize) {
        // Load bitmap
        let folder = "test_resources/regions/connected_components";
        let path = format!("{folder}/{filename}");
        let color_map: RgbaMap = RgbaField::load(path).unwrap().into();

        // Compute regions
        let (region_map, region_covers) = pixmap_regions(&color_map);

        assert_eq!(
            region_covers.len(),
            count,
            "number of components is correct"
        );

        // Make sure for each side, if left and right have the same color, they belong to the
        // same region.
        for (pixel, color) in color_map.iter() {
            for side in pixel.sides_ccw() {
                if Some(color) == color_map.get(side.right_pixel()) {
                    assert_eq!(region_map.get(pixel), region_map.get(side.right_pixel()));
                }
            }
        }
    }

    #[test]
    fn components_1a() {
        assert_proper_components("1a.png", 1);
    }

    #[test]
    fn components_2a() {
        assert_proper_components("2a.png", 2);
    }

    #[test]
    fn components_3a() {
        assert_proper_components("3a.png", 3);
    }

    #[test]
    fn components_3b() {
        assert_proper_components("3b.png", 3);
    }

    #[test]
    fn components_3c() {
        assert_proper_components("3c.png", 3);
    }

    #[test]
    fn components_3d() {
        assert_proper_components("3d.png", 3);
    }

    #[test]
    fn components_4a() {
        assert_proper_components("4a.png", 4);
    }

    #[test]
    fn components_5a() {
        assert_proper_components("5a.png", 4);
    }

    #[test]
    fn components_7a() {
        assert_proper_components("7a.png", 6);
    }
}
