use crate::{
    math::{pixel::Pixel, rect::Rect},
    pixmap::Pixmap,
    utils::IteratorPlus,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AreaBounds {
    /// Bounding rect of the area covered by the pixels. A pixel has an area of 1x1 with its center
    /// offset by 1/2, 1/2.
    bounds: Rect<i64>,
}

impl AreaBounds {
    /// Is the bounding rect returned by the current implementation exact? Might change in the
    /// future and make certain changes necessary.
    pub const EXACT_BOUNDING_RECT: bool = true;

    pub fn new() -> Self {
        AreaBounds {
            bounds: Rect::EMPTY,
        }
    }

    pub fn add_rect(&mut self, rect: Rect<i64>) {
        self.bounds = self.bounds.bounds_with_rect(rect);
    }

    pub fn add_pixel(&mut self, pixel: Pixel) {
        self.add_rect(Rect::low_size(pixel.point(), [1, 1]));
    }

    pub fn add_pixmap<T>(&mut self, pixmap: &Pixmap<T>) {
        self.add_rect(pixmap.actual_bounds());
    }

    pub fn add_bounds(&mut self, area_bounds: &AreaBounds) {
        self.add_rect(area_bounds.bounds);
    }

    pub fn bounding_rect(&self) -> Rect<i64> {
        self.bounds
    }

    pub fn iter(&self) -> impl IteratorPlus<Pixel> {
        self.bounds.iter_half_open().map(|index| index.into())
    }
}
