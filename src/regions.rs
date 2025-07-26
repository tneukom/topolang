use crate::{
    math::{
        pixel::{Pixel, Side, SideName},
        rect::Rect,
    },
    pixmap::Pixmap,
};
use ahash::HashSet;
use itertools::Itertools;
use std::hash::Hash;

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

fn pop_arbitrary<T: Eq + Hash + Copy>(set: &mut HashSet<T>) -> Option<T> {
    let &next = set.iter().next()?;
    Some(set.take(&next).unwrap())
}

/// Split a set of sides into cycles. Each cycle starts with its smallest side. The list of cycles
/// is ordered by the first side in each cycle. This means the first cycle is the outer cycle.
#[inline(never)]
pub fn split_boundary_into_cycles(mut sides: HashSet<Side>) -> Vec<Vec<Side>> {
    let mut cycles = Vec::new();

    while let Some(start_side) = pop_arbitrary(&mut sides) {
        // Extract cycle
        let mut cycle = vec![start_side];
        let mut side = start_side;
        'outer: loop {
            // If 3 regions meet in a corner, there are 2 possible continuing side, but only
            // one valid one. Because `side.continuing_sides()` returns the side in clockwise order
            // we get the right one. Careful when changing this code!
            for continuing_side in side.continuing_sides() {
                if continuing_side == start_side {
                    // Cycle finished
                    break 'outer;
                }

                if sides.remove(&continuing_side) {
                    cycle.push(continuing_side);
                    side = continuing_side;
                    continue 'outer;
                }
            }

            unreachable!("Has to continue with a side");
        }

        // Make sure cycle starts with the smallest element
        let i_min = cycle.iter().position_min().unwrap();
        cycle.rotate_left(i_min);

        cycles.push(cycle);
    }

    // Sort cycles by first element
    cycles.sort_by_key(|cycle| cycle[0]);
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
    Rect::index_bounds(boundary.map(|side| side.left_pixel))
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
        field::RgbaField,
        math::{pixel::Side, point::Point, rect::Rect, rgba8::Rgba8},
        pixmap::{Pixmap, RgbaMap},
        regions::{area_left_of_boundary, area_right_of_boundary, split_boundary_into_cycles},
    };
    use ahash::HashSet;
    use itertools::Itertools;
    use std::collections::{BTreeMap, BTreeSet};

    /// Contains interior and boundary sides
    pub fn iter_sides_in_rect(rect: Rect<i64>) -> impl Iterator<Item = Side> + Clone {
        let pixel_iter = rect.iter_indices();
        pixel_iter.flat_map(|pixel| pixel.sides_ccw().into_iter())
    }

    /// Collect the boundary of the area with the given value, in other words the sides that have
    /// a pixel of the given value on the left side and a different value on the right side.
    fn boundary_of<T: Ord + Copy>(map: &Pixmap<T>, interior: T) -> BTreeSet<Side> {
        let mut boundary = BTreeSet::new();
        for side in iter_sides_in_rect(map.bounding_rect()) {
            if map.get(side.left_pixel) == Some(interior) {
                let right_color = map.get(side.right_pixel());
                if right_color != Some(interior) {
                    boundary.insert(side);
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

    fn area_boundary(area: &HashSet<Point<i64>>) -> HashSet<Side> {
        area.iter()
            // iter over all sides of each pixel in area
            .flat_map(|pixel| pixel.sides_ccw())
            // keep only sides with a different region on the right side
            .filter(|side| !area.contains(&side.right_pixel()))
            .collect()
    }

    fn test_left_of_border(area: HashSet<Point<i64>>, n_sides: usize) {
        let boundary = area_boundary(&area);
        assert_eq!(boundary.len(), n_sides);

        let borders = split_boundary_into_cycles(boundary);
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
        let cycles_ccw = split_boundary_into_cycles(sides_ccw);
        assert_eq!(cycles_ccw.len(), 1);

        let sides_cw: HashSet<_> = Point(0, 0).sides_cw().into_iter().collect();
        let cycles_cw = split_boundary_into_cycles(sides_cw);
        assert_eq!(cycles_cw.len(), 1);

        assert_eq!(
            cycles_cw[0][0].start_corner(),
            cycles_ccw[0][0].start_corner()
        );
    }

    #[test]
    fn simple_left_of_border_a() {
        let area: HashSet<_> = [Point(0, 0)].into_iter().collect();
        test_left_of_border(area, 6);
    }

    #[test]
    fn simple_left_of_border_b() {
        let area: HashSet<_> = [Point(0, 0), Point(1, 0)].into_iter().collect();
        test_left_of_border(area, 10);
    }

    #[test]
    fn simple_left_of_border_c() {
        let area: HashSet<_> = [Point(0, 0), Point(1, 0), Point(0, 1), Point(1, 1)]
            .into_iter()
            .collect();
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

    /// Check if the area right of the red area inner border is equal to the blue area.
    fn test_right_of_border(filename: &str) {
        let folder = "test_resources/regions/right_of_border";
        let path = format!("{folder}/{filename}");
        let color_map: RgbaMap = RgbaField::load(path).unwrap().into();
        let red_area = pixmap_color_area(&color_map, Rgba8::RED);
        let blue_area = pixmap_color_area(&color_map, Rgba8::BLUE);

        let red_boundary = area_boundary(&red_area);
        let red_borders = split_boundary_into_cycles(red_boundary);
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
}
