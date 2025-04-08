use crate::{
    line_drawing::{circular_brush, draw_line_slope, rectangle_with_center},
    material::Material,
    math::{arrow::Arrow, pixel::Pixel, point::Point},
    pixmap::MaterialMap,
};
use ahash::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Brush {
    pub material: Material,
    pub size: i64,
}

impl Brush {
    pub fn default() -> Self {
        Self {
            material: Material::BLACK,
            size: 1,
        }
    }

    pub fn draw_line(&self, line: Arrow<f64>) -> HashMap<Pixel, Material> {
        let mut result = HashMap::default();
        for line_pixel in draw_line_slope(line, self.size) {
            result.insert(line_pixel, self.material);
        }

        result
    }

    /// Should be same as `self.draw_line(Arrow::new(center, center))
    pub fn dot(&self, center: Point<f64>) -> MaterialMap {
        let bounds = rectangle_with_center(center, Point(self.size, self.size));
        let mut material_map = MaterialMap::nones(bounds);

        for pixel in bounds.iter_indices() {
            if circular_brush(bounds.as_f64().center(), 0.5 * self.size as f64, pixel) {
                material_map.set(pixel, self.material);
            }
        }

        material_map
    }
}
