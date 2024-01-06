// #![feature(try_blocks)]
// #![feature(try_trait_v2 )]
#![allow(dead_code)]
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
// #![allow(unsafe_code)]

use crate::{app::EguiApp, compiler::Compiler, rule::stabilize, topology::Topology};

mod app;
mod array_2d;
mod bitmap;
mod brush;
mod camera;
mod compiler;
mod connected_components;
mod coordinate_frame;
mod math;
mod morphism;
mod painting;
mod pattern;
mod pixmap;
mod reduce;
mod rule;
mod serialize;
mod topology;
mod utils;
mod view;
mod widgets;

pub fn main_benchmark() {
    let folder = "test_resources/compiler/b/";
    let world = Topology::from_bitmap_path(format!("{folder}/world.png")).unwrap();

    let compiler = Compiler::new().unwrap();

    for _ in 0..100 {
        use std::time::Instant;

        let mut world = world.clone();

        let now = Instant::now();
        let rules = compiler.compile(&mut world).unwrap();
        println!("Compiled elapsed = {:.3?}", now.elapsed());

        let now = Instant::now();
        let application_count = stabilize(&mut world, &rules);
        println!(
            "Run, application_count = {application_count}, elapsed = {:.3?}",
            now.elapsed()
        );
    }
}

pub fn main_editor() {
    unsafe {
        let native_options = eframe::NativeOptions::default();
        let result = eframe::run_native(
            "SeamLang",
            native_options,
            Box::new(|cc| Box::new(EguiApp::new(cc))),
        );

        if result.is_err() {
            println!("Run failed");
        }
    }
}

pub fn main() {
    main_editor()

    // main_benchmark();
}
