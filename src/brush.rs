use crate::{
    math::{pixel::Pixel, point::Point, rgba8::Rgba8},
    pixmap::Pixmap,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Brush {
    pub color: Rgba8,
    pub radius: usize,
}

impl Brush {
    pub fn default() -> Self {
        Self {
            color: Rgba8::BLACK,
            radius: 1,
        }
    }

    pub fn draw_point(&self, target: &mut Pixmap, point: Point<f64>) {
        let pixel = Pixel::containing(point);
        target.set(pixel, self.color);
    }
}
