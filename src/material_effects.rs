use crate::{
    field::RgbaField,
    material::{Material, MaterialClass},
    math::{point::Point, rgba8::Rgba8},
    pixmap::MaterialMap,
};

enum BorderClass {
    Border,
    Gap,
    Inside,
}

/// Classify where `pixel` lies in an area defined by the `area` predicate. Assumes `area(pixel)`.
fn border_class(pixel: Point<i64>, mut area: impl FnMut(Point<i64>) -> bool) -> BorderClass {
    // const GAP_NEIGHBORS: [Point<i64>; 4] = [Point(-1, 0), Point(0, 1), Point(1, 0), Point(0, -1)];
    // const BORDER_NEIGHBORS: [Point<i64>; 8] = [
    //     Point(-2, 0),
    //     Point(-1, 1),
    //     Point(0, 2),
    //     Point(1, 1),
    //     Point(2, 0),
    //     Point(1, -1),
    //     Point(0, -2),
    //     Point(-1, -1),
    // ];

    const GAP_NEIGHBORS: [Point<i64>; 8] = [
        Point(-1, -1),
        Point(-1, 0),
        Point(-1, 1),
        Point(0, 1),
        Point(1, 1),
        Point(1, 0),
        Point(1, -1),
        Point(0, -1),
    ];
    const BORDER_NEIGHBORS: [Point<i64>; 16] = [
        Point(-2, -2),
        Point(-2, -1),
        Point(-2, 0),
        Point(-2, 1),
        Point(-2, 2),
        Point(-1, 2),
        Point(0, 2),
        Point(1, 2),
        Point(2, 2),
        Point(2, 1),
        Point(2, 0),
        Point(2, -1),
        Point(2, -2),
        Point(1, -2),
        Point(0, -2),
        Point(-1, -2),
    ];

    if GAP_NEIGHBORS
        .into_iter()
        .any(|offset| !area(pixel + offset))
    {
        BorderClass::Gap
    } else if BORDER_NEIGHBORS
        .into_iter()
        .any(|offset| !area(pixel + offset))
    {
        BorderClass::Border
    } else {
        BorderClass::Inside
    }
}

/// Convert Material to Rgba8 using effects for rule and solid areas
pub fn material_effect(material_map: &MaterialMap, pixel: Point<i64>) -> Rgba8 {
    let material = material_map.get(pixel).unwrap();

    match material.class {
        MaterialClass::Solid => {
            // alternating lines effect
            if pixel.y % 2 == 0 {
                material.solid_main_rgba()
            } else {
                material.solid_alt_rgba()
            }
        }
        MaterialClass::Rule => {
            // Border with gap effect
            let border_class = border_class(pixel, |neighbor| match material_map.get(neighbor) {
                None => false,
                Some(material) => material.is_rule(),
            });
            let alpha = match border_class {
                BorderClass::Border => Material::RULE_BORDER_ALPHA,
                BorderClass::Gap => Material::RULE_GAP_ALPHA,
                BorderClass::Inside => Material::RULE_INTERIOR_ALPHA,
            };

            Rgba8::from_rgb_a(material.rgb, alpha)
        }
        MaterialClass::Wildcard => {
            // alternating diagonal lines effect
            if (pixel.x + pixel.y) % 3 == 0 {
                Rgba8::from_rgb_a(Material::WILDCARD_ALT_RGB, Material::WILDCARD_ALPHA)
            } else {
                Rgba8::from_rgb_a(Material::WILDCARD_RGB, Material::WILDCARD_ALPHA)
            }
        }
        _ => material.to_rgba(),
    }
}

pub fn material_map_effects(material_map: &MaterialMap, rgba_field: &mut RgbaField) {
    assert_eq!(material_map.bounding_rect(), rgba_field.bounds());

    for pixel in material_map.keys() {
        let effect_rgba = material_effect(material_map, pixel);
        rgba_field.set(pixel, effect_rgba);
    }
}

#[cfg(test)]
mod test {
    use crate::{
        field::RgbaField, material_effects::material_map_effects, math::rgba8::Rgba8,
        pixmap::MaterialMap,
    };

    #[test]
    fn test_apply_material_effects() {
        let mut material_map =
            MaterialMap::load("test_resources/material_effects/gates4.png").unwrap();

        let mut rgba_field = RgbaField::filled(material_map.bounding_rect(), Rgba8::ZERO);
        material_map_effects(&material_map, &mut rgba_field);

        rgba_field
            .save("test_resources/material_effects/gates4_effects.png")
            .unwrap();
    }
}
