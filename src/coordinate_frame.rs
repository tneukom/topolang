// See https://registry.khronos.org/OpenGL-Refpages/gl4/html/glViewport.xhtml for relation between
// normalized device coordinates and window coordinates.

// OpenGL normalized device coordinates (device coordinates for short)
// -1,1         1,1
//  ┌────────────┐
//  │            │
//  │            │
//  └────────────┘
// -1,-1        1,-1

// OpenGL window coordinates (window coordinates for short)
// 0,0          w,0
//  ┌────────────┐
//  │            │
//  │            │
//  └────────────┘
// 0,h          w,h
// Center of window frame is at w/h, h/2

// View coordinates is same as OpenGL window coordinates

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

    pub fn window_center(self) -> Point<f64> {
        Point(self.width as f64, self.height as f64) / 2.0
    }

    pub fn view_center(self) -> Point<f64> {
        self.window_to_view() * self.window_center()
    }

    pub fn window_to_device(self) -> AffineMap<f64> {
        AffineMap::map_points(
            Point(0.0, 0.0),
            Point(-1.0, 1.0),
            Point(self.width as f64, 0.0),
            Point(1.0, 1.0),
            Point(0.0, self.height as f64),
            Point(-1.0, -1.0),
        )
    }

    pub fn device_to_window(self) -> AffineMap<f64> {
        self.window_to_device().inv()
    }

    pub fn view_to_window(self) -> AffineMap<f64> {
        AffineMap::ID
    }

    pub fn window_to_view(self) -> AffineMap<f64> {
        self.view_to_window().inv()
    }

    pub fn view_to_device(self) -> AffineMap<f64> {
        self.window_to_device() * self.view_to_window()
    }

    pub fn device_to_view(self) -> AffineMap<f64> {
        self.view_to_device().inv()
    }
}
