// #![feature(try_blocks)]
// #![feature(try_trait_v2 )]
#![allow(dead_code)]
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
// #![allow(unsafe_code)]

use crate::{
    bitmap::Bitmap, connected_components::color_dict_from_bitmap, seam_graph::SeamGraph,
    topology::Topology,
};

mod array_2d;
mod bitmap;
mod connected_components;
mod math;
mod reduce;
mod seam_graph;
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
    let pixelmap = color_dict_from_bitmap(&bitmap);
    let topology = Topology::new(&pixelmap);
    println!("{topology}");
    let seam_graph = SeamGraph::from_topology(&topology);
    println!("{seam_graph}");
}
