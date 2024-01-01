// #![feature(try_blocks)]
// #![feature(try_trait_v2 )]
#![allow(dead_code)]
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
// #![allow(unsafe_code)]

use crate::{bitmap::Bitmap, pixmap::Pixmap, topology::Topology};

mod array_2d;
mod bitmap;
mod compiler;
mod connected_components;
mod math;
mod morphism;
mod pattern;
mod pixmap;
mod reduce;
mod rule;
mod serialize;
mod topology;
mod utils;

pub fn main() {
    // TODO:
    // - [ ] Load bitmap
    // - [ ] Connected components
    // - [ ] Topology

    let path = "test_resources/topology/3b.png";
    let bitmap = Bitmap::from_path(path).unwrap();
    let pixmap = Pixmap::from_bitmap(&bitmap);
    let topology = Topology::new(&pixmap);
    println!("{topology}");
}
