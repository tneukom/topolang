use crate::{
    math::{arrow::Arrow, pixel::Pixel, point::Point, rect::Rect, rgba8::Rgba8},
    pixmap::Pixmap,
};

#[derive(Debug, Clone, Copy)]
pub struct Brush {
    pub color: Rgba8,
    pub radius: f64,
}

impl Brush {
    pub fn default() -> Self {
        Self {
            color: Rgba8::BLACK,
            radius: 0.5_f64.sqrt(), // half diagonal
        }
    }

    pub fn draw_point(&self, target: &mut Pixmap, point: Point<f64>) {
        let pixel = Pixel::containing(point);
        target.set(pixel, self.color);
    }

    pub fn draw_line(&self, target: &mut Pixmap, line: Arrow<f64>) {
        for point in Self::points_within_radius(line, self.radius) {
            target.set(point.into(), self.color);
        }
    }

    /// Slow but flexible line drawing, only use for small lines!
    /// FIXME: Find something faster than iterating over bounding box
    /// FIXME: Use trait alias for return trait when stable
    pub fn points_within_radius(
        line: Arrow<f64>,
        radius: f64,
    ) -> impl Iterator<Item = Point<i64>> + IntoIterator<Item = Point<i64>> + Clone {
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
