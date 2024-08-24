/// Unionfind crates
/// https://crates.io/crates/union-find
/// https://crates.io/crates/disjoint-sets
///
/// https://crates.io/crates/partitions
/// Has clear function
///
/// https://docs.rs/petgraph/latest/src/petgraph/unionfind.rs.html#16-27
use crate::{
    area_cover::AreaCover,
    field::Field,
    math::{
        pixel::{Side, SideName},
        point::Point,
        rgba8::Rgba8,
    },
    pixmap::{Pixmap, Tile, TileSideClass, TileSidesForEach},
    union_find::UnionFind,
};
use ahash::{HashMap, HashMapExt};
use itertools::Itertools;

use std::{collections::BTreeMap, rc::Rc};

pub trait RegionEq {
    fn region_eq(self, other: Self) -> bool;
}

impl<T: RegionEq> RegionEq for Option<T> {
    fn region_eq(self, other: Self) -> bool {
        match (self, other) {
            (Some(this), Some(other)) => this.region_eq(other),
            (None, None) => true,
            _ => false,
        }
    }
}

impl RegionEq for () {
    fn region_eq(self, _other: Self) -> bool {
        true
    }
}

impl RegionEq for Rgba8 {
    fn region_eq(self, other: Self) -> bool {
        self == other
    }
}

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

impl<T: Copy + RegionEq> NeighborsFn<T> for BuildRegionsUnionFind {
    // Build UnionFind data structure for equivalent regions
    fn call<const N: usize>(&mut self, field: &Field<T>, center: usize, neighbors: [usize; N]) {
        let center_value = field.as_slice()[center];
        for neighbor in neighbors {
            let neighbor_value = field.as_slice()[neighbor];
            if neighbor_value.region_eq(center_value) {
                self.union_find.union(center, neighbor);
            }
        }
    }
}

#[inline(never)]
pub fn field_regions_fast<T: Copy + RegionEq>(field: &Field<T>) -> Field<usize> {
    let union_find = UnionFind::new(field.len());
    let mut build_union_find = BuildRegionsUnionFind { union_find };
    build_union_find.for_each(field);
    let labeling = build_union_find.union_find.into_roots();
    Field::from_linear(field.bounds(), labeling)
}

/// Returns connected components of `field`
#[inline(never)]
pub fn field_regions<T: Copy + RegionEq>(field: &Field<T>) -> Field<usize> {
    // TODO: Reuse UnionFind so we don't allocate each time
    let mut union_find = UnionFind::new(field.len());

    for (center, &color) in field.enumerate() {
        for side in [SideName::TopLeft, SideName::Top, SideName::Left] {
            let neighbor = center.neighbor(side);
            if let Some(neighbor_color) = field.get(neighbor).copied() {
                if color.region_eq(neighbor_color) {
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
pub fn tile_regions<T: Copy + RegionEq>(tile: &Tile<T>) -> (Tile<usize>, usize) {
    let mut region_field = field_regions_fast(&tile.field);
    let mut region_field = mask_field(&mut region_field, &tile.field);

    // TODO: Reuse CompactLabels so we don't allocate each time
    let mut compact_labels = CompactLabels::new(region_field.len());
    compact_labels.compact_masked(region_field.iter_mut());
    (Tile::from_field(region_field), compact_labels.counter)
}

/// Returns the region map and the number of regions
#[inline(never)]
pub fn pixmap_regions<T: Copy + RegionEq>(
    color_map: &Pixmap<T>,
) -> (Pixmap<usize>, Vec<AreaCover>) {
    // Map pixmap tile wise (not regarding interface between tiles)

    // Maps each region_id to the tile it is in
    let mut region_to_tile: Vec<Point<i64>> = Vec::new();

    let region_map_tiles: BTreeMap<_, _> = color_map
        .tiles
        .iter()
        .map(|(&tile_index, tile)| {
            // Compute regions of the tile independent of the rest of the pixmap
            let (mut tile_region_map, tile_n_regions) = tile_regions(tile);

            // Offset region ids
            for (_, region_id) in tile_region_map.iter_mut() {
                *region_id += region_to_tile.len();
            }

            // Map region_id to tile_index
            for _ in 0..tile_n_regions {
                region_to_tile.push(tile_index);
            }

            (tile_index, Rc::new(tile_region_map))
        })
        .collect();
    let mut region_map = Pixmap {
        tiles: region_map_tiles,
    };

    // Build UnionFind to merge tiled regions that are connected
    let mut union_find = UnionFind::new(region_to_tile.len());

    let tile_sides_for_each = TileSidesForEach::cached();
    tile_sides_for_each.for_each_zip(
        color_map,
        &region_map,
        TileSideClass::Boundary,
        |_side, left, right| {
            if let Some(right) = right {
                let (left_color, right_color) = (left.0, right.0);
                if left_color.region_eq(right_color) {
                    union_find.union(left.1, right.1);
                }
            }
        },
    );

    let mut roots = union_find.into_roots();

    // Make sure labels are in range 0,...,n_regions
    let mut compact_labels = CompactLabels::new(region_to_tile.len());
    let n_regions = compact_labels.compact(&mut roots);
    for (_, region_id) in region_map.iter_mut() {
        *region_id = roots[*region_id]
    }

    // Create AreaCover for each region
    let mut area_covers = vec![AreaCover::new(); n_regions];
    for (region_id, &tile_index) in region_to_tile.iter().enumerate() {
        let merged_region_id = roots[region_id];
        area_covers[merged_region_id].add_tile(tile_index);
    }

    (region_map, area_covers)
}

pub type RegionBoundary = HashMap<Side, Option<usize>>;

#[inline(never)]
pub fn region_boundaries(region_map: &Pixmap<usize>, n_regions: usize) -> Vec<RegionBoundary> {
    let mut boundaries = vec![RegionBoundary::new(); n_regions];

    let tile_sides_for_each = TileSidesForEach::cached();
    tile_sides_for_each.for_each(region_map, TileSideClass::All, |side, left, right| {
        let boundary = &mut boundaries[left];
        if Some(left) != right {
            boundary.insert(side, right);
        }
    });

    // region_map.for_each_side_neighbors(|side, neighbors| {
    //     let boundary = &mut boundaries[*neighbors.left];
    //     if Some(neighbors.left) != neighbors.right {
    //         boundary.insert(side, neighbors.right.cloned());
    //     }
    // });

    boundaries
}

/// Split a set of sides into cycles. Each cycle starts with its smallest side. The list of cycles
/// is ordered by the first side in each cycle. This means the first cycle is the outer cycle.
#[inline(never)]
pub fn split_boundary_into_cycles<T>(mut sides: HashMap<Side, T>) -> Vec<Vec<(Side, T)>> {
    let mut cycles = Vec::new();

    while !sides.is_empty() {
        // Pop first element
        let mut side = *sides.keys().next().unwrap();
        let color = sides.remove(&side).unwrap();

        // Extract cycle
        let mut cycle = vec![(side, color)];
        'outer: loop {
            for next_side in side.continuing_sides() {
                // There is always exactly one continuing side
                if let Some(next_color) = sides.remove(&next_side) {
                    cycle.push((next_side, next_color));
                    side = next_side;
                    continue 'outer;
                }
            }

            break;
        }

        // Make sure cycle starts with smallest element
        let i_min = cycle.iter().position_min_by_key(|(side, _)| side).unwrap();
        cycle.rotate_left(i_min);

        cycles.push(cycle);
    }

    // Sort cycles by first element
    cycles.sort_by_key(|cycle| cycle[0].0);
    cycles
}

// pub enum Malleability  {
//     Solid,
//     Pliable
// }

// TODO: Might make more sense to have a PixSet class
// pub fn solid_regions(material_map: &MaterialMap) -> Pixmap<Material> {
//     let solid_map = material_map.filter(|_, material| material.is_solid() )
// }

#[cfg(test)]
mod test {
    use crate::{
        field::Field,
        math::{generic::EuclidDivRem, pixel::Side, point::Point, rect::Rect, rgba8::Rgba8},
        pixmap::{iter_sides_in_rect, split_index, MaterialMap, Pixmap, RgbaMap, TILE_SIZE},
        regions::{pixmap_regions, region_boundaries},
        utils::IntoT,
    };
    use ahash::{HashMap, HashMapExt};
    use itertools::Itertools;
    use std::collections::BTreeMap;

    pub fn pixel_touches_tile_boundary(index: Point<i64>) -> bool {
        let (_, pixel_index) = split_index(index);
        pixel_index.x == 0
            || pixel_index.x == TILE_SIZE - 1
            || pixel_index.y == 0
            || pixel_index.y == TILE_SIZE - 1
    }

    pub fn side_lies_on_tile_boundary(side: Side) -> bool {
        pixel_touches_tile_boundary(side.left_pixel)
    }

    fn generate_field(bounds: Rect<i64>) -> Field<Rgba8> {
        Field::from_map(bounds, |index| {
            let red = index.x.euclid_rem(256) as u8;
            let green = index.y.euclid_rem(256) as u8;
            Rgba8::new(red, green, 255, 255)
        })
    }

    fn field_bounds_cases() -> Vec<Rect<i64>> {
        vec![
            Rect::low_high([0, 0], [1, 1]),
            Rect::low_high([-1, -1], [0, 0]),
            Rect::low_high([0, 0], [TILE_SIZE - 1, TILE_SIZE - 1]),
            Rect::low_high([0, 0], [TILE_SIZE + 1, TILE_SIZE - 1]),
            Rect::low_high([-391, 234], [519, 748]),
        ]
    }

    #[test]
    fn convert_field() {
        for bounds in field_bounds_cases() {
            let field = generate_field(bounds);
            let pixmap = Pixmap::from_field(&field);
            assert_eq!(pixmap.bounding_rect(), bounds);

            for (index, value) in field.enumerate() {
                assert_eq!(Some(value), pixmap.get(index));
            }

            assert_eq!(pixmap.to_field(Rgba8::ZERO), field);
        }
    }

    /// Collect the boundary of the area with the given value, in other words the sides that have
    /// a pixel of the given value on the left side and a different value on the right side.
    fn boundary_of<T: Ord + Clone>(map: &Pixmap<T>, interior: &T) -> HashMap<Side, Option<T>> {
        let mut boundary = HashMap::new();
        for side in iter_sides_in_rect(map.bounding_rect()) {
            if map.get(side.left_pixel) == Some(interior) {
                let right_color = map.get(side.right_pixel());
                if right_color != Some(interior) {
                    boundary.insert(side, right_color.cloned());
                }
            }
        }
        boundary
    }

    /// Compute region map from color map, given that there are no two regions with the same
    /// color.
    fn pixmap_region_given_distinct_colors(color_map: &Pixmap<Rgba8>) -> Pixmap<usize> {
        let mut color_to_region = BTreeMap::new();
        let mut region_map = Pixmap::new();
        for (pixel, &color) in color_map.iter() {
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
        for &region_id in region_map.values() {
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

    #[test]
    fn regions_multiple_tiles() {
        // Images must be updated if TILE_SIZE changes!
        const {
            assert!(TILE_SIZE == 64);
        }
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
            let expected_boundary = boundary_of(&region_map, &region_id);
            assert_eq!(boundary, &expected_boundary);
        }
    }

    #[test]
    fn region_boundaries_a() {
        test_region_boundaries("a.png");
    }

    #[test]
    fn region_boundaries_multiple_tiles() {
        // Images must be updated if TILE_SIZE changes!
        const {
            assert!(TILE_SIZE == 64);
        }
        test_region_boundaries("multiple_tiles_a.png");
        test_region_boundaries("multiple_tiles_b.png");
        test_region_boundaries("multiple_tiles_c.png");
    }
}
