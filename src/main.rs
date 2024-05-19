// #![allow(dead_code)]
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use log::warn;
use seamlang::{app::EguiApp, field::RgbaField, math::rgba8::Rgba8};
use walkdir::WalkDir;

// pub fn main_benchmark() {
//     use seamlang::{compiler::Compiler, rule::stabilize, topology::Topology};
//
//     let folder = "test_resources/compiler/b/";
//     let world = Topology::from_bitmap_path(format!("{folder}/world.png")).unwrap();
//
//     let compiler = Compiler::new().unwrap();
//
//     for _ in 0..100 {
//         use std::time::Instant;
//
//         let mut world = world.clone();
//
//         let now = Instant::now();
//         let rules = compiler.compile(&mut world).unwrap();
//         println!("Compiled elapsed = {:.3?}", now.elapsed());
//
//         let now = Instant::now();
//         let application_count = stabilize(&mut world, &rules);
//         println!(
//             "Run, application_count = {application_count}, elapsed = {:.3?}",
//             now.elapsed()
//         );
//     }
// }

pub fn color_replace() {
    for entry in WalkDir::new("./").into_iter().filter_map(Result::ok) {
        let extension = entry
            .path()
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        if extension.as_deref() != Some("png") {
            continue;
        }

        // Load png
        let Ok(bitmap) = RgbaField::load(entry.path()) else {
            println!("Failed to load {:?}", entry.path());
            continue;
        };

        let legacy_rule_frame_color = Rgba8::from_hex("360C29B4").unwrap();
        // alpha = 180 = 0xB4
        let rule_frame_color = Rgba8::from_hex("360C29B4").unwrap();
        let legacy_rule_arrow_color = Rgba8::from_hex("FF6E00B4").unwrap();
        // alpha = 181 = 0xB5
        let rule_arrow_color = Rgba8::from_hex("FF6E00B5").unwrap();

        // Replace colors
        let result = bitmap.map(|&color| {
            if color == legacy_rule_frame_color {
                rule_frame_color
            } else if color == legacy_rule_arrow_color {
                rule_arrow_color
            } else {
                color
            }
        });

        if result != bitmap {
            println!("Saving {:?}", entry.path());
            if let Err(_) = result.save(entry.path()) {
                println!("Failed to save {:?}", entry.path());
            }
        } else {
            println!("{:?} not modified", entry.path());
        }

        println!("Processed file {:?}", entry.path());
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn main_editor() {
    unsafe {
        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_drag_and_drop(true),
            ..eframe::NativeOptions::default()
        };
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        warn!("Logging!");
        main_editor();
        // color_replace();
    }

    // main_benchmark();
}
