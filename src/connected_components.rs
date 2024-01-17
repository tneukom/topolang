use crate::{
    math::{
        pixel::{Pixel, Side, SideName},
        rgba8::Rgba8,
    },
    pixmap::Pixmap,
};
use std::collections::BTreeSet;

pub struct ConnectedComponent {
    /// Cannot be empty
    pub interior: BTreeSet<Pixel>,

    /// Cannot be empty
    pub sides: BTreeSet<Side>,
    pub closed: bool,
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

        for side_name in SideName::ALL {
            let neighbor_pixel = pixel.neighbor(side_name);
            let neighbor_side = pixel.side_ccw(side_name);

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

pub fn color_components(pixels: &Pixmap) -> Vec<ColorComponent> {
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

// pub fn component_map<'a>(components: impl Iterator<Item = &'a ConnectedComponent>) -> BTreeMap<Pixel, usize> {
//     let mut map = BTreeMap::new();
//     for (i, component) in components.enumerate() {
//         for &pixel in &component.interior {
//             let before = map.insert(pixel, i);
//             assert!(before.is_none())
//         }
//     }
//
//     map
// }

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
    use std::collections::{BTreeSet, HashSet};
    // TODO: Make sure color of pixels in components is constant
    use crate::{
        bitmap::Bitmap,
        connected_components::{color_components, left_of, ColorComponent, ConnectedComponent},
        pixmap::Pixmap,
    };

    fn assert_proper_components(filename: &str, count: usize) {
        let folder = "test_resources/connected_components";
        let path = format!("{folder}/{filename}");

        let bitmap = Bitmap::from_path(path).unwrap();
        let whole = Pixmap::from_bitmap(&bitmap);
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
    fn assert_total_union(components: &Vec<ColorComponent>, whole: &Pixmap) {
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
        assert_proper_components("5a.png", 4);
    }

    #[test]
    fn components_7a() {
        assert_proper_components("7a.png", 6);
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
            let whole = Pixmap::from_bitmap(&bitmap);
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
