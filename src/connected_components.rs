use crate::{
    bitmap::Bitmap,
    math::{
        direction::Direction,
        pixel::{Pixel, Side},
        rgba8::Rgba8,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub enum Interior {
    Bounded(BTreeSet<Pixel>),
    Unbounded(),
}

pub struct ConnectedComponent {
    pub interior: BTreeSet<Pixel>,
    pub sides: BTreeSet<Side>,
    pub closed: bool,
}

pub enum SideClass {
    Interior,
    Border,
    Open,
}

pub fn flood_fill(
    seed: Pixel,
    classify: impl Fn(&Side, &Pixel) -> SideClass,
) -> ConnectedComponent {
    let mut interior: BTreeSet<Pixel> = BTreeSet::new();
    let mut sides: BTreeSet<Side> = BTreeSet::new();

    let mut todo: Vec<Pixel> = vec![seed];
    let mut closed = true;

    while !todo.is_empty() {
        let pixel = todo.pop().unwrap();

        if !interior.insert(pixel) {
            // was already contained in interior
            continue;
        }

        for direction in Direction::ALL {
            let neighbor_pixel = pixel.neighbor(direction);
            let neighbor_side = pixel.side_ccw(direction);

            match classify(&neighbor_side, &neighbor_pixel) {
                SideClass::Border => {
                    sides.insert(neighbor_side);
                }
                SideClass::Interior => {
                    todo.push(neighbor_pixel);
                }
                SideClass::Open => {
                    sides.insert(neighbor_side);
                    closed = false;
                }
            }
        }
    }

    ConnectedComponent {
        interior,
        sides,
        closed,
    }
}

pub struct ColorComponent {
    pub component: ConnectedComponent,
    pub color: Rgba8,
}

pub fn color_components(pixels: &BTreeMap<Pixel, Rgba8>) -> Vec<ColorComponent> {
    let mut rest: BTreeSet<_> = pixels.keys().cloned().collect();
    let mut color_components: Vec<ColorComponent> = Vec::new();

    while !rest.is_empty() {
        let &seed = rest.iter().next().unwrap();
        let &seed_color = pixels.get(&seed).unwrap();

        let classify = |_: &Side, neighbor: &Pixel| {
            if let Some(&neighbor_color) = pixels.get(neighbor) {
                if neighbor_color == seed_color {
                    SideClass::Interior
                } else {
                    SideClass::Border
                }
            } else {
                SideClass::Open
            }
        };
        let component = flood_fill(seed, classify);

        // Remove component interior from rest
        for pixel in &component.interior {
            rest.remove(pixel);
        }

        let color_component = ColorComponent {
            component,
            color: seed_color,
        };

        color_components.push(color_component);
    }

    color_components
}

pub fn pixmap_from_bitmap(bitmap: &Bitmap) -> BTreeMap<Pixel, Rgba8> {
    let mut dict: BTreeMap<Pixel, Rgba8> = BTreeMap::new();
    for idx in bitmap.indices() {
        let color = bitmap[idx];

        if color.a == 0 && color != Rgba8::TRANSPARENT {
            println!("Bitmap should not contain colors with alpha = 0 but rgb != 0")
        }

        let pixel: Pixel = idx.cwise_try_into::<i64>().unwrap().into();
        dict.insert(pixel, color);
    }
    dict
}

pub fn pixmap_from_path(path: impl AsRef<Path>) -> anyhow::Result<BTreeMap<Pixel, Rgba8>> {
    let bitmap = Bitmap::from_path(path)?;
    Ok(pixmap_from_bitmap(&bitmap))
}

/// Works if `boundary` is the boundary of a connected set of pixels, can contain holes.
fn flood_fill_border(seed: Pixel, boundary: &BTreeSet<Side>) -> ConnectedComponent {
    let classify = |side: &Side, _: &Pixel| {
        if boundary.contains(side) || boundary.contains(&side.reversed()) {
            SideClass::Border
        } else {
            SideClass::Interior
        }
    };
    flood_fill(seed, classify)
}

/// See flood_fill_border
pub fn left_of(boundary: &BTreeSet<Side>) -> ConnectedComponent {
    let seed = boundary.first().unwrap().left_pixel();
    flood_fill_border(seed, boundary)
}

/// See flood_fill_border
pub fn right_of(boundary: &BTreeSet<Side>) -> ConnectedComponent {
    let seed = boundary.first().unwrap().right_pixel();
    flood_fill_border(seed, boundary)
}

#[cfg(test)]
mod test {
    use std::collections::{BTreeMap, BTreeSet, HashSet};
    // TODO: Make sure color of pixels in components is constant
    use crate::{
        bitmap::Bitmap,
        connected_components::{
            color_components, left_of, pixmap_from_bitmap, ColorComponent, ConnectedComponent,
        },
        math::{pixel::Pixel, rgba8::Rgba8},
    };

    fn assert_proper_components(filename: &str, count: usize) {
        let folder = "test_resources/connected_components";
        let path = format!("{folder}/{filename}");

        let bitmap = Bitmap::from_path(path).unwrap();
        let whole = pixmap_from_bitmap(&bitmap);
        let components = color_components(&whole);
        assert_eq!(components.len(), count, "number of components is correct");

        for ColorComponent { component, color } in &components {
            // Components cannot be empty
            assert_ne!(component.interior.len(), 0);

            // Make sure the color of each pixel in the component is the same
            assert!(component
                .interior
                .iter()
                .all(|pixel| whole[pixel] == *color));

            // Make sure the boundary of each component is proper
            assert_proper_sides(component);
        }

        assert_total_union(&components, &whole);

        // Make sure the total number of pixels in all components is the same as the number of
        // pixels in the whole. Together with assert_total_union this implies non intersection.
        let components_pixel_count: usize = components
            .iter()
            .map(|comp| comp.component.interior.len())
            .sum();
        assert_eq!(components_pixel_count, whole.len());
    }

    // Assert that the union of all component interiors is equal to the whole
    fn assert_total_union(components: &Vec<ColorComponent>, whole: &BTreeMap<Pixel, Rgba8>) {
        let union: HashSet<_> = components
            .iter()
            .flat_map(|comp| comp.component.interior.iter().cloned())
            .collect();
        let whole_pixels = whole.keys().cloned().collect();
        assert_eq!(union, whole_pixels);
    }

    ///
    fn assert_proper_sides(component: &ConnectedComponent) {
        if !component.closed {
            // The boundary has gaps so left_of(sides) is not defined.
            return;
        }

        let all_sides: HashSet<_> = component
            .interior
            .iter()
            .flat_map(|pixel| pixel.sides_ccw())
            .collect();

        let boundary_sides: BTreeSet<_> = all_sides
            .into_iter()
            .filter(|side| {
                let left_interior = component.interior.contains(&side.left_pixel());
                let right_interior = component.interior.contains(&side.right_pixel());
                left_interior != right_interior
            })
            .collect();

        assert_eq!(
            boundary_sides, component.sides,
            "component.sides = boundary(interior)"
        );
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
        assert_proper_components("5a.png", 5);
    }

    #[test]
    fn components_7a() {
        assert_proper_components("7a.png", 7);
    }

    #[test]
    fn left_of_component() {
        let folder = "test_resources/connected_components";
        let filenames = [
            "2a.png", "3a.png", "3b.png", "3c.png", "3d.png", "4a.png", "5a.png", "7a.png",
        ];

        for filename in filenames {
            let path = format!("{folder}/{filename}");
            let bitmap = Bitmap::from_path(path).unwrap();
            let whole = pixmap_from_bitmap(&bitmap);
            let components = color_components(&whole);

            for ColorComponent { component, color } in components {
                let connected_component = left_of(&component.sides);
                assert_eq!(
                    connected_component.interior, component.interior,
                    "{filename} failed for component {color}"
                );
            }
        }
    }
}
