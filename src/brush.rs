use crate::{
    field::{Field, MaterialField},
    line_drawing::{circular_brush, draw_line_slope, rectangle_with_center},
    material::Material,
    math::{arrow::Arrow, pixel::Pixel, point::Point},
};
use ahash::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct Brush {
    pub material: Material,
    pub width: i64,
}

impl Brush {
    pub fn default() -> Self {
        Self {
            material: Material::BLACK,
            width: 1,
        }
    }

    pub fn draw_line(&self, line: Arrow<f64>) -> HashMap<Pixel, Material> {
        let mut result = HashMap::default();
        for line_pixel in draw_line_slope(line, self.width as f64) {
            result.insert(line_pixel, self.material);
        }

        result
    }

    /// Should be same as `self.draw_line(Arrow::new(center, center))
    pub fn dot(&self, center: Point<f64>) -> MaterialField {
        let bounds = rectangle_with_center(center, Point(self.width, self.width));
        Field::from_map(bounds, |pixel| {
            if circular_brush(center, self.width as f64, pixel) {
                self.material
            } else {
                Material::TRANSPARENT
            }
        })
    }
}
