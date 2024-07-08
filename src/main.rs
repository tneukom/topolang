// #![allow(dead_code)]
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use log::warn;
use seamlang::{
    app::EguiApp,
    connected_components::{color_components, ColorRegion},
    field::{Field, RgbaField},
    math::rgba8::Rgba8,
    pixmap::{Pixmap, RgbaMap},
    regions::{
        field_regions, field_regions2, field_regions4, field_regions4b,
        field_regions5, pixmap_regions, pixmap_regions2, Pixmap2,
    },
};
use std::time::{Duration, Instant};
use walkdir::WalkDir;
use seamlang::regions::{field_regions2b, field_regions2c, field_regions2d, field_regions2q, field_regions4c};

pub fn main_benchmark_field_regions() {
    let folder = "test_resources/regions";
    let color_map = Field::load(format!("{folder}/b.png")).unwrap();

    let mut total_elapsed = Duration::from_millis(0);
    for _ in 0..1000 {
        let now = Instant::now();
        let region_map = field_regions2(&color_map);
        let elapsed = now.elapsed();
        total_elapsed += elapsed;
        println!("Elapsed = {:.3?}", now.elapsed());
    }
    println!("Total elapsed = {:.3?}", total_elapsed);

    // field_regions4b: 4.791s
    // field_regions2: 4.175s
    // field_regions: 11.999s
    // field_regions4: 1.563s
    // field_regions5: 7.782s

    let region_map = field_regions2(&color_map);
    let region_map_rgba = region_map.map(|id| Rgba8::new(*id as u8, 0, 0, 255));
    region_map_rgba.save(format!("{folder}/b_out.png")).unwrap();
}

pub fn main_benchmark_regions() {
    let folder = "test_resources/regions";
    let color_map: Pixmap2<Rgba8> = Field::load(format!("{folder}/b.png")).unwrap().into();

    for _ in 0..100 {
        let now = Instant::now();
        let region_map = pixmap_regions2(&color_map);
        println!("Elapsed = {:.3?}", now.elapsed());
    }
}

pub fn main_benchmark_legacy_regions() {
    fn usize_color_components(
        color_map: &RgbaMap,
    ) -> (Vec<ColorRegion<usize, Rgba8>>, Pixmap<usize>) {
        let mut id: usize = 0;
        let free_id = || {
            let result = id;
            id += 1;
            result
        };
        color_components(color_map, free_id)
    }

    let folder = "test_resources/regions";
    let color_map: Pixmap<Rgba8> = Field::load(format!("{folder}/b.png")).unwrap().to_pixmap();

    for _ in 0..100 {
        let now = Instant::now();
        let region_map = usize_color_components(&color_map);
        println!("Legacy method elapsed = {:.3?}", now.elapsed());
    }
}

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
        // main_editor();
        // color_replace();

        main_benchmark_field_regions();
    }
}
