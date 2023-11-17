// #![feature(try_blocks)]
// #![feature(try_trait_v2 )]
#![allow(dead_code)]
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(unsafe_code)]

use crate::math::{point::Point, rgba8::Rgba8};
use egui::ahash::HashSet;
use std::collections::HashMap;

mod array_2d;
mod bitmap;
mod connected_components;
mod math;
mod reduce;
mod serialize;
mod topology;
mod utils;

struct Solid {
    pub pixels: Vec<Point<i64>>,
}

struct Topology {
    pub solids: Vec<Solid>,
}

pub fn main() {
    // TODO:
    // - [ ] Load bitmap
    // - [ ] Connected components
    // - [ ] Topology
}
