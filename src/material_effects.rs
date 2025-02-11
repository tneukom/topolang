use crate::{
    field::Field,
    math::{point::Point, rect::Rect},
    pixmap::Pixmap,
    regions::{area_left_of_boundary, area_right_of_boundary},
    topology::{Border, Boundary},
};

pub fn border_pixels(interior: &Field<bool>) -> impl Iterator<Item = Point<i64>> + '_ {
    interior.enumerate().filter_map(|(pixel, &inside)| {
        let has_outside_neighbor = pixel
            .neighbors()
            .into_iter()
            .any(|neighbor| !interior.get(neighbor).copied().unwrap_or(false));
        (inside && has_outside_neighbor).then_some(pixel)
    })
}

/// Remove the outermost pixels and return them.
pub fn contract(interior: &mut Field<bool>) -> Vec<Point<i64>> {
    let border: Vec<_> = border_pixels(interior).collect();
    for &pixel in &border {
        interior.set(pixel, false);
    }
    border
}

fn border_interior_field(border: &Border, bounds: Rect<i64>) -> Field<bool> {
    assert!(border.is_outer);

    let mut interior = Field::filled(bounds, false);
    for pixel in area_left_of_boundary(border.sides()) {
        interior.set(pixel, true);
    }
    interior
}

pub fn border_fade_effect<T: Copy>(
    mut interior: Field<bool>,
    border_colors: &[T],
    interior_color: T,
) -> Pixmap<T> {
    let mut pixmap = Pixmap::nones(interior.bounds());
    for &color in border_colors {
        let border = contract(&mut interior);
        for pixel in border {
            pixmap.set(pixel, color);
        }
    }

    // Fill what's left of the interior.
    for (pixel, &inside) in interior.enumerate() {
        if inside {
            pixmap.set(pixel, interior_color);
        }
    }

    pixmap
}

pub fn boundary_fade_effect<T: Copy>(
    boundary: &Boundary,
    border_colors: &[T],
    interior_color: T,
) -> Pixmap<T> {
    let interior = border_interior_field(boundary.outer_border(), boundary.interior_bounds);
    let mut effect = border_fade_effect(interior, border_colors, interior_color);

    // Clear holes
    for hole in boundary.holes() {
        for pixel in area_right_of_boundary(hole.sides()) {
            effect.remove(pixel);
        }
    }

    effect
}

pub fn alternating_pattern_effect<T: Copy>(
    pixmap: &mut Pixmap<T>,
    area: impl IntoIterator<Item = Point<i64>>,
    color_even: T,
    color_odd: T,
) {
    for pixel in area.into_iter() {
        let is_even = (pixel.x + pixel.y) % 2 == 0;
        let color = if is_even { color_even } else { color_odd };
        pixmap.set(pixel, color);
    }
}

#[cfg(test)]
mod test {
    use crate::{
        field::RgbaField,
        material::Material,
        material_effects::{alternating_pattern_effect, border_fade_effect, boundary_fade_effect},
        math::rgba8::Rgba8,
        pixmap::{MaterialMap, Pixmap},
        topology::Topology,
        utils::KeyValueItertools,
    };

    #[test]
    fn test_border_fade_effect() {
        let area = RgbaField::load("test_resources/material_effects/area_0.png").unwrap();
        let interior = area.map(|&color| color == Rgba8::RED);

        let alphas = [255, 200, 150];
        let border_colors = alphas.map(|alpha| Rgba8::new(255, 0, 0, alpha));
        let interior_color = Rgba8::new(255, 0, 0, 100);

        let effect = border_fade_effect(interior, &border_colors, interior_color);
        effect
            .save("test_resources/material_effects/area_0_border_fade_effect.png")
            .unwrap();
    }

    #[test]
    fn test_boundary_fade_effect() {
        let material_map = MaterialMap::load("test_resources/material_effects/area_1.png").unwrap();
        let topology = Topology::new(material_map.clone());

        let red_region = topology
            .regions
            .values()
            .find(|region| region.material == Material::RED)
            .unwrap();

        let alphas = [255, 200, 150];
        let border_materials =
            alphas.map(|alpha| Material::from_rgba(Rgba8::new(255, 0, 0, alpha)));
        let interior_material = Material::from_rgba(Rgba8::new(255, 0, 0, 100));

        let effect =
            boundary_fade_effect(&red_region.boundary, &border_materials, interior_material);

        effect
            .save("test_resources/material_effects/area_1_border_fade_effect.png")
            .unwrap();
    }

    #[test]
    fn test_alternating_pattern_effect() {
        let material_map = MaterialMap::load("test_resources/material_effects/area_0.png").unwrap();
        let mut effect = Pixmap::nones_like(&material_map);
        let topology = Topology::new(material_map);

        let &region_key = topology
            .regions
            .iter()
            .find_key_by_value(|region| region.material == Material::RED)
            .unwrap();

        let material_even = Material::BLUE;
        let material_odd = Material::GREEN;
        alternating_pattern_effect(
            &mut effect,
            topology.iter_region_interior(region_key),
            material_even,
            material_odd,
        );

        effect
            .save("test_resources/material_effects/area_0_alternating_effect.png")
            .unwrap();
    }
}
