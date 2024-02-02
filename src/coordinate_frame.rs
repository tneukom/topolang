// Glwindow reference frame
// -1,1         1,1
//  ┌────────────┐
//  │            │
//  │            │
//  └────────────┘
// -1,-1        1,-1

// Pixelwindow (same as View) reference frame
// 0,0          w,0
//  ┌────────────┐
//  │            │
//  │            │
//  └────────────┘
// 0,h          w,h
// Center of Glpixel refernce frame is at w/h, h/2

use crate::math::{affine_map::AffineMap, point::Point};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CoordinateFrames {
    width: i64,
    height: i64,
}

impl CoordinateFrames {
    pub fn new(width: i64, height: i64) -> CoordinateFrames {
        CoordinateFrames { width, height }
    }

    pub fn pixelwindow_center(self) -> Point<f64> {
        Point(self.width as f64, self.height as f64) / 2.0
    }

    pub fn view_center(self) -> Point<f64> {
        self.pixelwindow_to_view() * self.pixelwindow_center()
    }

    pub fn pixelwindow_to_glwindow(self) -> AffineMap<f64> {
        AffineMap::map_points(
            Point(0.0, 0.0),
            Point(-1.0, 1.0),
            Point(self.width as f64, 0.0),
            Point(1.0, 1.0),
            Point(0.0, self.height as f64),
            Point(-1.0, -1.0),
        )
    }

    pub fn glwindow_to_pixelwindow(self) -> AffineMap<f64> {
        self.pixelwindow_to_glwindow().inv()
    }

    pub fn view_to_pixelwindow(self) -> AffineMap<f64> {
        AffineMap::ID
    }

    pub fn pixelwindow_to_view(self) -> AffineMap<f64> {
        self.view_to_pixelwindow().inv()
    }

    pub fn view_to_glwindow(self) -> AffineMap<f64> {
        self.pixelwindow_to_glwindow() * self.view_to_pixelwindow()
    }

    pub fn glwindow_to_view(self) -> AffineMap<f64> {
        self.view_to_glwindow().inv()
    }
}
