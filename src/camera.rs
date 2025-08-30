use crate::math::{affine_map::AffineMap, point::Point, rect::Rect};

#[derive(Debug, Clone, Copy)]
/// An affine map that maps points from the view to the world coordinate system.
/// `view_to_world`
pub struct Camera {
    pub offset: Point<f64>,

    /// Scale of world_from_view or 1/scale of view_from_world. Meaning a smaller scale is more
    /// zoomed in.
    pub scale: f64,
}

impl Camera {
    const ZOOM_LEVELS: [f64; 17] = [
        1.0 / 16.0,
        1.0 / 12.0,
        1.0 / 8.0,
        1.0 / 6.0,
        1.0 / 5.0,
        1.0 / 4.0,
        1.0 / 3.0,
        1.0 / 2.0,
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        8.0,
        12.0,
        16.0,
    ];

    fn new(offset: Point<f64>, scale: f64) -> Self {
        Camera { offset, scale }
    }

    /// Round the offset to the nearest multiple of scale
    pub fn round(&self) -> Self {
        let offset = (self.offset / self.scale).round() * self.scale;
        Self::new(offset, self.scale)
    }

    pub fn world_from_view(&self) -> AffineMap<f64> {
        AffineMap::similarity(self.scale, self.offset)
    }

    pub fn view_from_world(&self) -> AffineMap<f64> {
        self.world_from_view().inv()
    }

    pub fn default() -> Camera {
        Camera::new(Point::ZERO, 1.0)
    }

    // The resulting camera maps view_point to world_point with the given scale
    pub fn map_view_to_world(
        view_point: Point<f64>,
        world_point: Point<f64>,
        scale: f64,
    ) -> Camera {
        // world_point = scale * view_point + offset
        let offset = world_point - view_point * scale;
        Camera::new(offset, scale)
    }

    // Zoom out at the given point in the view reference frame.
    // The result maps `view_point` to `view_to_world * view_point`
    pub fn zoom_out_at_view_point(&self, view_point: Point<f64>) -> Camera {
        let Some(next_scale) = Self::ZOOM_LEVELS
            .into_iter()
            .filter(|&zoom| zoom > self.scale)
            .next()
        else {
            return *self;
        };

        let world_point = self.world_from_view() * view_point;
        Self::map_view_to_world(view_point, world_point, next_scale)
    }

    /// See `zoom_out_at_view_point`
    pub fn zoom_in_at_view_point(&self, view_point: Point<f64>) -> Camera {
        let Some(next_scale) = Self::ZOOM_LEVELS
            .into_iter()
            .filter(|&zoom| zoom < self.scale)
            .last()
        else {
            return *self;
        };

        let world_point = self.world_from_view() * view_point;
        Self::map_view_to_world(view_point, world_point, next_scale)
    }

    /// A camera that maps the center of world_rect to the center of view_rect and
    /// camera.world_to_view * world_rect fits into view_rect
    /// scale is power of 2
    pub fn fit_world_into_view(world_rect: Rect<f64>, view_rect: Rect<f64>) -> Camera {
        // scale * view_rect.width() >= world_rect.width() and scale * view_rect.height() >= world_rect.height()
        let scale =
            (world_rect.width() / view_rect.width()).max(world_rect.height() / view_rect.height());

        // round scale down to power of 2 because we currently don't have Rational rounding
        // TODO: Use .round()
        let mut rounded_scale = 0.5_f64.powi(32);
        while rounded_scale < scale {
            rounded_scale = rounded_scale * 2.0;
        }

        // world_center = scale * view_center + offset
        let offset = world_rect.center() - view_rect.center() * rounded_scale;

        //Camera::new(offset, rounded_scale)
        let result = Camera {
            offset,
            scale: rounded_scale,
        };
        result
    }
}
