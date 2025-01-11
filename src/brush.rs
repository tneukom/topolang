use crate::{
    line_drawing::draw_line_slope,
    material::Material,
    math::{arrow::Arrow, pixel::Pixel},
};
use ahash::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct Brush {
    pub material: Material,
    pub radius: i64,
}

impl Brush {
    pub fn default() -> Self {
        Self {
            material: Material::BLACK,
            radius: 1,
        }
    }

    pub fn draw_line(&self, line: Arrow<f64>) -> HashMap<Pixel, Material> {
        let mut result = HashMap::default();
        for line_pixel in draw_line_slope(line, self.radius as f64) {
            result.insert(line_pixel, self.material);
        }

        result
    }
}
