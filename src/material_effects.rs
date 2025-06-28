use crate::{
    field::RgbaField,
    material::{Material, MaterialClass},
    math::{
        point::Point,
        rgba8::{Rgb8, Rgba8},
    },
    pixmap::MaterialMap,
};

enum BorderClass {
    Border,
    Gap,
    Inside,
}

const NEIGHBORS_8: [Point<i64>; 8] = [
    Point(-1, -1),
    Point(-1, 0),
    Point(-1, 1),
    Point(0, 1),
    Point(1, 1),
    Point(1, 0),
    Point(1, -1),
    Point(0, -1),
];

// const NEIGHBORS_4: [Point<i64>; 4] = [Point(-1, 0), Point(0, 1), Point(1, 0), Point(0, -1)];

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

    const GAP_NEIGHBORS: [Point<i64>; 8] = NEIGHBORS_8;

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

// /// Border and gap
// pub fn rule_effect(material_map: &MaterialMap, pixel: Point<i64>, material: Material) -> Rgba8 {
//     // Border with gap effect
//     let border_class = border_class(pixel, |neighbor| match material_map.get(neighbor) {
//         None => false,
//         Some(material) => material.is_rule(),
//     });
//     let alpha = match border_class {
//         BorderClass::Border => Material::RULE_BORDER_ALPHA,
//         BorderClass::Gap => Material::RULE_GAP_ALPHA,
//         BorderClass::Inside => Material::RULE_INTERIOR_ALPHA,
//     };
//
//     Rgba8::from_rgb_a(material.rgb, alpha)
// }

/// Border to transparent color
pub fn rule_effect(material_map: &MaterialMap, pixel: Point<i64>, material: Material) -> Rgba8 {
    let is_border = NEIGHBORS_8.into_iter().any(|neighbor_offset| {
        match material_map.get(pixel + neighbor_offset) {
            None => true,
            Some(neighbor_material) => neighbor_material == Material::TRANSPARENT,
        }
    });

    let alpha = if is_border {
        Material::RULE_BORDER_ALPHA
    } else {
        Material::RULE_INTERIOR_ALPHA
    };

    Rgba8::from_rgb_a(material.rgb, alpha)
}

pub fn repeating_effect<T, const PATTERN_WIDTH: usize, const PATTERN_HEIGHT: usize>(
    pixel: Point<i64>,
    on: T,
    off: T,
    pattern: &[[bool; PATTERN_WIDTH]; PATTERN_HEIGHT],
) -> T {
    let pattern_offset = pixel.cwise_rem_euclid(Point(PATTERN_WIDTH as i64, PATTERN_HEIGHT as i64));
    let alternative = pattern[pattern_offset.y as usize][pattern_offset.x as usize];
    if alternative {
        on
    } else {
        off
    }
}

pub fn sleep_effect(pixel: Point<i64>, rgb: Rgb8) -> Rgba8 {
    // ██ ███ ███ █
    //  ███ ███ ███
    //
    // const PATTERN_WIDTH: usize = 4;
    // const PATTERN_HEIGHT: usize = 3;
    // const PATTERN: [[bool; PATTERN_WIDTH]; PATTERN_HEIGHT] = [
    //     [true, true, false, true],
    //     [false, true, true, true],
    //     [false, false, false, false],
    // ];

    //  █  █  █  █
    // █  █  █  █
    //   █  █  █  █
    // const PATTERN_WIDTH: usize = 3;
    // const PATTERN_HEIGHT: usize = 3;
    // const PATTERN: [[bool; PATTERN_WIDTH]; PATTERN_HEIGHT] = [
    //     [false, true, false],
    //     [true, false, false],
    //     [false, false, true],
    // ];

    //  ██  ██  ██  ██
    // ██  ██  ██  ██
    // █  ██  ██  ██  █
    //   ██  ██  ██  ██
    const PATTERN_WIDTH: usize = 4;
    const PATTERN_HEIGHT: usize = 4;
    const PATTERN: [[bool; PATTERN_WIDTH]; PATTERN_HEIGHT] = [
        [false, true, true, false],
        [true, true, false, false],
        [true, false, false, true],
        [false, false, true, true],
    ];

    // ███ ███ ███ ███
    // █   █   █   █
    // █ ███ ███ ███ ██
    //   █   █   █   █
    // const PATTERN_WIDTH: usize = 4;
    // const PATTERN_HEIGHT: usize = 4;
    // const PATTERN: [[bool; PATTERN_WIDTH]; PATTERN_HEIGHT] = [
    //     [true, true, true, false],
    //     [true, false, false, false],
    //     [true, false, true, true],
    //     [false, false, true, false],
    // ];

    let alpha = repeating_effect(
        pixel,
        Material::SLEEPING_ALT_ALPHA,
        Material::SLEEPING_ALPHA,
        &PATTERN,
    );
    Rgba8::from_rgb_a(rgb, alpha)
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
        MaterialClass::Rule => rule_effect(material_map, pixel, material),
        MaterialClass::Wildcard => {
            // alternating diagonal lines effect
            let k = (pixel.x + pixel.y).rem_euclid(Material::WILDCARD_RAINBOW_RGB.len() as i64);
            let rgb = Material::WILDCARD_RAINBOW_RGB[k as usize];
            Rgba8::from_rgb_a(rgb, Material::WILDCARD_ALPHA)
        }
        MaterialClass::Sleeping => sleep_effect(pixel, material.rgb),
        _ => material.to_rgba(),
    }
}

pub fn material_map_effects(material_map: &MaterialMap, background: Rgba8) -> RgbaField {
    let mut field = RgbaField::filled(material_map.bounding_rect(), background);
    paint_material_map_effects(material_map, &mut field);
    field
}

pub fn paint_material_map_effects(material_map: &MaterialMap, rgba_field: &mut RgbaField) {
    assert_eq!(material_map.bounding_rect(), rgba_field.bounds());

    for pixel in material_map.keys() {
        let effect_rgba = material_effect(material_map, pixel);
        rgba_field.set(pixel, effect_rgba);
    }
}

pub const CHECKERBOARD_EVEN_RGBA: Rgba8 = Rgba8::new(0xB2, 0xB2, 0xB2, 0xFF);
pub const CHECKERBOARD_ODD_RGBA: Rgba8 = Rgba8::new(0x99, 0x99, 0x99, 0xFF);

#[cfg(test)]
mod test {
    use crate::{material_effects::material_map_effects, math::rgba8::Rgba8, pixmap::MaterialMap};

    #[test]
    fn test_apply_material_effects() {
        let material_map = MaterialMap::load("test_resources/material_effects/gates4.png").unwrap();

        let rgba_field = material_map_effects(&material_map, Rgba8::ZERO);

        rgba_field
            .save("test_resources/material_effects/gates4_effects.png")
            .unwrap();
    }
}
