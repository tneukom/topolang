// #![allow(dead_code)]
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use log::warn;
use seamlang::{
    app::EguiApp,
    field::{Field, RgbaField},
    interpreter::Interpreter,
    math::rgba8::Rgba8,
    pixmap::{MaterialMap, RgbaMap},
    regions::{field_regions_fast, pixmap_regions, CompactLabels},
    topology::Topology,
    utils::IntoT,
    world::World,
};
use std::time::{Duration, Instant};
use walkdir::WalkDir;

pub fn main_benchmark_field_regions() {
    let folder = "test_resources/regions";
    let color_map = Field::load(format!("{folder}/b.png")).unwrap();

    let mut total_elapsed = Duration::from_millis(0);
    let mut compact_labels = CompactLabels::new(color_map.len());
    for _ in 0..1000 {
        let now = Instant::now();
        let region_map = field_regions_fast(&color_map);
        // compact_labels.clear();
        // compact_labels.compact(region_map.iter_mut());

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

    let mut region_map = field_regions_fast(&color_map);
    compact_labels.clear();
    compact_labels.compact(region_map.iter_mut());
    let region_map_rgba = region_map.map(|id| Rgba8::new(*id as u8, 0, 0, 255));
    region_map_rgba.save(format!("{folder}/b_out.png")).unwrap();
}

pub fn main_benchmark_pixmap_regions() {
    let folder = "test_resources/regions";
    let color_field = Field::load(format!("{folder}/b.png")).unwrap();
    let color_map: RgbaMap = color_field.into();

    let mut total_elapsed = Duration::from_millis(0);
    for _ in 0..500 {
        let now = Instant::now();
        let region_map = pixmap_regions(&color_map);
        let elapsed = now.elapsed();
        total_elapsed += elapsed;
        println!("Elapsed = {:.3?}", now.elapsed());
    }

    let (region_map, _) = pixmap_regions(&color_map);
    let region_field = region_map.to_field(255);
    let region_field_rgba = region_field.map(|id| Rgba8::new(*id as u8, 0, 0, 255));
    region_field_rgba
        .save(format!("{folder}/b_out.png"))
        .unwrap();
}

pub fn main_benchmark() {
    let folder = "resources/saves";
    let original_world = RgbaField::load(format!("{folder}/turing.png"))
        .unwrap()
        .intot::<MaterialMap>()
        .intot::<World>();

    let interpreter = Interpreter::new();

    for _ in 0..100 {
        use std::time::Instant;

        let mut world = original_world.clone();

        let now = Instant::now();
        let mut steps = 0usize;
        loop {
            steps += 1;
            let changed = interpreter.step(&mut world);
            if !changed {
                break;
            }
        }
        println!("steps = {}, elapsed = {:.3?}", steps, now.elapsed());
    }
}

pub fn benchmark_topology_new() {
    let folder = "resources/saves";
    let material_map = RgbaField::load(format!("{folder}/turing.png"))
        .unwrap()
        .intot::<MaterialMap>();

    for _ in 0..100 {
        use std::time::Instant;
        let now = Instant::now();
        Topology::new(material_map.clone());
        println!("elapsed = {:.3?}", now.elapsed());
    }
}

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
        let legacy_rule_arrow_color = Rgba8::from_hex("FF6E00B5").unwrap();
        // alpha = 181 = 0xB5
        let rule_arrow_color = Rgba8::from_hex("FF6E00B4").unwrap();

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
        // main_benchmark();
        // benchmark_topology_new();
        main_editor();
        // color_replace();

        // main_benchmark_pixmap_regions();

        // main_benchmark_legacy_regions();
        // main_benchmark_pixmap_regions();
        // main_benchmark_field_regions();
    }
}
