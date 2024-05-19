use std::collections::HashMap;

use crate::{
    material::Material,
    math::{arrow::Arrow, pixel::Pixel, point::Point, rect::Rect},
    pixmap::{MaterialMap, Pixmap},
    utils::IteratorPlus,
};

#[derive(Debug, Clone, Copy)]
pub struct Brush {
    pub material: Material,
    pub radius: i64,
}

impl Brush {
    pub fn default() -> Self {
        Self {
            material: Material::BLACK,
            radius: 0,
        }
    }

    /// Circle
    fn stamp(radius: i64) -> Vec<Point<i64>> {
        Rect::low_high([-radius, -radius], [radius, radius])
            .iter_closed()
            .filter(|&p| p.norm_squared() <= radius * radius)
            .collect()
    }

    pub fn draw_line(&self, line: Arrow<f64>) -> MaterialMap {
        // for point in Self::points_within_radius(line, self.radius) {
        //     target.set(point.into(), self.color);
        // }

        let stamp = Self::stamp(self.radius);

        let pixel_line: Arrow<i64> = Arrow(
            line.a.floor().cwise_into_lossy(),
            line.b.floor().cwise_into_lossy(),
        );

        let mut result = HashMap::new();
        for point in pixel_line.draw() {
            for &offset in &stamp {
                result.insert(Pixel::from(point + offset), self.material);
            }
        }

        Pixmap::from_hashmap(&result)
    }

    /// Slow but flexible line drawing, only use for small lines!
    /// FIXME: Find something faster than iterating over bounding box
    pub fn points_within_radius(line: Arrow<f64>, radius: f64) -> impl IteratorPlus<Point<i64>> {
        let bbox = line.bounds().padded(radius.ceil());
        let low = bbox.low().floor().cwise_into_lossy::<i64>();
        let high = bbox.high().ceil().cwise_into_lossy::<i64>();

        Rect::low_high(low, high).iter_closed().filter(move |p| {
            let f64_p = p.cwise_into_lossy::<f64>();
            let distance_squared = line.distance_squared(f64_p);
            distance_squared < radius * radius
        })
    }
}
