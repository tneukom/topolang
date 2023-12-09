use crate::{
    bitmap::Bitmap,
    math::{
        direction::Direction,
        pixel::{Pixel, Side},
        rgba8::Rgba8,
    },
};
use std::collections::{BTreeMap, BTreeSet};

pub enum Interior {
    Bounded(BTreeSet<Pixel>),
    Unbounded(),
}

pub struct ConnectedComponent {
    pub interior: BTreeSet<Pixel>,
    pub sides: BTreeSet<Side>,
    pub color: Rgba8,
}

pub fn flood_fill(
    seed: Pixel,
    is_boundary: impl Fn(Pixel, Side) -> bool,
) -> (BTreeSet<Pixel>, BTreeSet<Side>) {
    let mut interior: BTreeSet<Pixel> = BTreeSet::new();
    let mut sides: BTreeSet<Side> = BTreeSet::new();

    let mut todo: Vec<Pixel> = vec![seed];

    while !todo.is_empty() {
        let pixel = todo.pop().unwrap();

        if interior.contains(&pixel) {
            continue;
        }

        interior.insert(pixel);

        for direction in Direction::ALL {
            let neighbor_pixel = pixel.neighbor(direction);
            let neighbor_side = pixel.side_ccw(direction);

            if is_boundary(neighbor_pixel, neighbor_side) {
                sides.insert(neighbor_side);
            } else {
                todo.push(neighbor_pixel);
            }
        }
    }

    (interior, sides)
}

pub fn connected_components(pixels: &BTreeMap<Pixel, Rgba8>) -> Vec<ConnectedComponent> {
    let mut rest: BTreeSet<_> = pixels.keys().cloned().collect();
    let mut components: Vec<ConnectedComponent> = Vec::new();

    while !rest.is_empty() {
        let &seed = rest.iter().next().unwrap();
        let seed_color = pixels.get(&seed).unwrap();

        let is_boundary = |pixel: Pixel, _side: Side| pixels.get(&pixel) != Some(seed_color);
        let (interior, sides) = flood_fill(seed, is_boundary);

        // Remove component interior from rest
        for pixel in &interior {
            rest.remove(pixel);
        }

        let component = ConnectedComponent {
            interior,
            sides,
            color: *seed_color,
        };
        components.push(component);
    }

    components
}

pub fn pixelmap_from_bitmap(bitmap: &Bitmap) -> BTreeMap<Pixel, Rgba8> {
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

#[cfg(test)]
mod test {
    use std::collections::{BTreeMap, BTreeSet, HashSet};
    // TODO: Make sure color of pixels in components is constant
    use crate::{
        bitmap::Bitmap,
        connected_components::{connected_components, pixelmap_from_bitmap, ConnectedComponent},
        math::{pixel::Pixel, rgba8::Rgba8},
        utils::all_equal,
    };

    /// Make sure the color of each pixel in the component is the same
    fn assert_constant_color(component: &ConnectedComponent, pixelmap: &BTreeMap<Pixel, Rgba8>) {
        let is_constant = all_equal(component.interior.iter().map(|pixel| pixelmap[pixel]));
        assert!(is_constant);
    }

    fn assert_proper_components(filename: &str, count: usize) {
        let folder = "test_resources/connected_components";
        let path = format!("{folder}/{filename}");

        let bitmap = Bitmap::from_path(path).unwrap();
        let whole = pixelmap_from_bitmap(&bitmap);
        let components = connected_components(&whole);
        assert_eq!(components.len(), count, "number of components is correct");

        for component in &components {
            // Components cannot be empty
            assert_ne!(component.interior.len(), 0);

            assert_constant_color(component, &whole);

            // Make sure the boundary of each component is proper
            assert_proper_sides(component);
        }

        assert_total_union(&components, &whole);

        // Make sure the total number of pixels in all components is the same as the number of
        // pixels in the whole. Together with assert_total_union this implies non intersection.
        let components_pixel_count: usize = components.iter().map(|comp| comp.interior.len()).sum();
        assert_eq!(components_pixel_count, whole.len());
    }

    // Assert that the union of all component interiors is equal to the whole
    fn assert_total_union(components: &Vec<ConnectedComponent>, whole: &BTreeMap<Pixel, Rgba8>) {
        let union: HashSet<_> = components
            .iter()
            .flat_map(|comp| comp.interior.iter().cloned())
            .collect();
        let whole_pixels = whole.keys().cloned().collect();
        assert_eq!(union, whole_pixels);
    }

    ///
    fn assert_proper_sides(component: &ConnectedComponent) {
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
}
