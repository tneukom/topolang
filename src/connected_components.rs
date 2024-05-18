use std::collections::BTreeSet;

use itertools::Itertools;

use crate::{
    area_cover::AreaCover,
    math::pixel::{Pixel, Side, SideName},
    pixmap::Pixmap,
};

pub struct ConnectedComponent {
    /// Cannot be empty
    pub interior: BTreeSet<Pixel>,

    /// Cannot be empty
    pub sides: BTreeSet<Side>,
}

impl ConnectedComponent {
    pub fn arbitrary_interior(&self) -> Pixel {
        *self.interior.first().unwrap()
    }

    pub fn arbitrary_side(&self) -> Side {
        *self.sides.first().unwrap()
    }
}

pub enum SideClass {
    Interior,
    Border,
    Open,
}

pub fn flood_fill<Id: Copy + Eq, T: Eq + Copy>(
    color_map: &Pixmap<T>,
    region_map: &mut Pixmap<Id>,
    cover: &mut AreaCover,
    seed: Pixel,
    region_id: Id,
) -> BTreeSet<Side> {
    let mut sides: BTreeSet<Side> = BTreeSet::new();
    let seed_color = color_map[seed];

    let previous = region_map.set(seed, region_id);
    cover.add(seed);
    assert!(previous.is_none());
    let mut todo: Vec<Pixel> = vec![seed];

    while let Some(pixel) = todo.pop() {
        for side_name in SideName::ALL {
            let neighbor_pixel = pixel.neighbor(side_name);
            let neighbor_side = pixel.side_ccw(side_name);

            if color_map.get(neighbor_pixel) == Some(&seed_color) {
                let previous = region_map.set(neighbor_pixel, region_id);
                cover.add(neighbor_pixel);
                if previous.is_none() {
                    todo.push(neighbor_pixel);
                } else if previous != Some(region_id) {
                    unreachable!();
                }
            } else {
                sides.insert(neighbor_side);
            }
        }
    }

    sides
}

pub struct ColorComponent<T> {
    pub component: ConnectedComponent,
    pub color: T,
}

pub struct ColorRegion<Id, T> {
    pub id: Id,
    pub color: T,
    pub sides: BTreeSet<Side>,
    pub cover: AreaCover,
}

/// `subset` must contain whole regions only
pub fn color_components_subset<Id: Eq + Copy, T: Eq + Copy>(
    color_map: &Pixmap<T>,
    subset: impl IntoIterator<Item = Pixel>,
    mut free_id: impl FnMut() -> Id,
) -> (Vec<ColorRegion<Id, T>>, Pixmap<Id>) {
    let mut regions: Vec<ColorRegion<Id, T>> = Vec::new();
    let mut region_map = Pixmap::new();

    for seed in subset.into_iter() {
        if region_map.get(seed).is_some() {
            continue;
        }

        let seed_color = color_map[seed];
        let id = free_id();
        let mut cover = AreaCover::new();
        let sides = flood_fill(color_map, &mut region_map, &mut cover, seed.into(), id);

        let region = ColorRegion {
            id,
            sides,
            cover,
            color: seed_color,
        };

        regions.push(region);
    }

    (regions, region_map)
}

pub fn color_components<Id: Eq + Copy, T: Eq + Copy>(
    color_map: &Pixmap<T>,
    free_id: impl FnMut() -> Id,
) -> (Vec<ColorRegion<Id, T>>, Pixmap<Id>) {
    color_components_subset(color_map, color_map.keys(), free_id)
}

// TODO: Use iterator instead of Vec<Side>
/// Requires border to be ccw, starting with left side and stopping with top side
pub fn left_of_border(border: &Vec<Side>) -> Vec<Pixel> {
    assert!(!border.is_empty());
    assert_eq!(border.first().unwrap().name, SideName::Left);
    assert_eq!(border.last().unwrap().name, SideName::Top);

    let mut left_right_sides: Vec<_> = border
        .iter()
        .copied()
        .filter(|side| side.name == SideName::Left || side.name == SideName::Right)
        .collect();
    left_right_sides.sort();

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

/// Split a set of sides into cycles. If a cycle is ccw it starts with a left side, if it is
/// cw it starts with a bottom side.
/// `[cycle.first() | cycle in cycles]` is ordered from small to large. This means the
/// first cycle is the outer cycle.
pub fn split_into_cycles<'a>(mut sides: BTreeSet<Side>) -> Vec<Vec<Side>> {
    let mut cycles = Vec::new();

    while let Some(mut side) = sides.pop_first() {
        // Extract cycle
        let mut cycle = vec![side];
        'outer: loop {
            for next_side in side.continuing_sides() {
                // There is always exactly one continuing side
                if sides.remove(&next_side) {
                    cycle.push(next_side);
                    side = next_side;
                    continue 'outer;
                }
            }
            // See side_ordering test in pixel.rs
            assert!([SideName::Left, SideName::Bottom].contains(&cycle.first().unwrap().name));
            // No continuing side still in sides, so the cycle is finished.
            break;
        }

        cycles.push(cycle);
    }

    cycles
}

pub fn right_of_border(border: &Vec<Side>) -> Vec<Pixel> where
{
    let reversed_border = border
        .into_iter()
        .rev()
        .map(|side| side.reversed())
        .collect();
    left_of_border(&reversed_border)
}

#[cfg(test)]
mod test {
    use std::collections::BTreeSet;

    // TODO: Make sure color of pixels in components is constant
    use crate::{
        connected_components::{
            color_components, left_of_border, right_of_border, split_into_cycles, ColorRegion,
        },
        math::{
            pixel::{Pixel, Side},
            rgba8::Rgba8,
        },
        pixmap::{Pixmap, PixmapRgba},
    };

    fn usize_color_components(
        color_map: &PixmapRgba,
    ) -> (Vec<ColorRegion<usize, Rgba8>>, Pixmap<usize>) {
        let mut id: usize = 0;
        let free_id = || {
            let result = id;
            id += 1;
            result
        };
        color_components(color_map, free_id)
    }

    fn area_boundary(area: &BTreeSet<Pixel>) -> BTreeSet<Side> {
        area.iter()
            // iter over all sides of each pixel in area
            .flat_map(|pixel| pixel.sides_ccw())
            // keep only sides with a different region on the right side
            .filter(|side| !area.contains(&side.right_pixel()))
            .collect()
    }

    fn pixmap_color_area<T: Eq>(pixmap: &Pixmap<T>, area_color: &T) -> BTreeSet<Pixel> {
        pixmap
            .iter()
            .filter(|(_, pixel_color)| pixel_color == &area_color)
            .map(|kv| kv.0)
            .collect()
    }

    /// Brute force method to find all boundary sides of a region given the region_map
    fn region_boundary(region_map: &Pixmap<usize>, id: usize) -> BTreeSet<Side> {
        let area = pixmap_color_area(region_map, &id);
        area_boundary(&area)
    }

    fn test_left_of_border(area: BTreeSet<Pixel>, n_sides: usize) {
        let boundary = area_boundary(&area);
        assert_eq!(boundary.len(), n_sides);

        let borders = split_into_cycles(boundary.into());
        assert_eq!(borders.len(), 1);

        let border = &borders[0];
        let left_of_border = left_of_border(border);
        assert_eq!(area, left_of_border.into_iter().collect());
    }

    #[test]
    fn simple_left_of_border_a() {
        let area: BTreeSet<_> = [Pixel::new(0, 0)].into();
        test_left_of_border(area, 6);
    }

    #[test]
    fn simple_left_of_border_b() {
        let area: BTreeSet<_> = [Pixel::new(0, 0), Pixel::new(1, 0)].into();
        test_left_of_border(area, 10);
    }

    #[test]
    fn simple_left_of_border_c() {
        let area: BTreeSet<_> = [
            Pixel::new(0, 0),
            Pixel::new(1, 0),
            Pixel::new(0, 1),
            Pixel::new(1, 1),
        ]
        .into();
        test_left_of_border(area, 14);
    }

    fn test_right_of_border(filename: &str) {
        let folder = "test_resources/connected_components/right_of_border";
        let path = format!("{folder}/{filename}");
        let color_map = PixmapRgba::load_bitmap(path).unwrap();
        let red_area = pixmap_color_area(&color_map, &Rgba8::RED);
        let blue_area = pixmap_color_area(&color_map, &Rgba8::BLUE);

        let red_boundary = area_boundary(&red_area);
        let red_borders = split_into_cycles(red_boundary.into());
        assert_eq!(red_borders.len(), 2); // inner and outer border

        let inner_border = &red_borders[1];
        let right_of_inner_border = right_of_border(&inner_border);
        let right_of_inner_border_set: BTreeSet<_> = right_of_inner_border.into_iter().collect();
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
        let folder = "test_resources/connected_components";
        let path = format!("{folder}/{filename}");
        let color_map = PixmapRgba::load_bitmap(path).unwrap();

        // Compute regions
        let (regions, region_map) = usize_color_components(&color_map);

        assert_eq!(regions.len(), count, "number of components is correct");

        // Make sure for each pixel, regions[region_map[pixel]].color == color_map[pixel]
        for (pixel, &color) in color_map.iter() {
            let &region_id = region_map.get(pixel).unwrap();
            assert_eq!(regions[region_id].color, color);
        }

        // Make sure for each side, if left and right have the same color, they belong to the
        // same region.
        for (pixel, color) in color_map.iter() {
            for side in pixel.sides_ccw() {
                if Some(color) == color_map.get(side.right_pixel()) {
                    assert_eq!(region_map.get(pixel), region_map.get(side.right_pixel()));
                }
            }
        }

        // Make sure sides of region are correct
        for region in &regions {
            let expected_sides = region_boundary(&region_map, region.id);
            assert_eq!(region.sides, expected_sides);
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
