// #![allow(dead_code)]
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use log::warn;

// pub fn color_replace() {
//     for entry in WalkDir::new("./").into_iter().filter_map(Result::ok) {
//         let extension = entry
//             .path()
//             .extension()
//             .and_then(|ext| ext.to_str())
//             .map(|ext| ext.to_lowercase());
//
//         if extension.as_deref() != Some("png") {
//             continue;
//         }
//
//         // Load png
//         let Ok(bitmap) = RgbaField::load(entry.path()) else {
//             println!("Failed to load {:?}", entry.path());
//             continue;
//         };
//
//         let legacy_rule_frame_color = Rgba8::from_hex("360C29B4").unwrap();
//         // alpha = 180 = 0xB4
//         let rule_frame_color = Rgba8::from_hex("360C29B4").unwrap();
//         let legacy_rule_arrow_color = Rgba8::from_hex("FF6E00B5").unwrap();
//         // alpha = 181 = 0xB5
//         let rule_arrow_color = Rgba8::from_hex("FF6E00B4").unwrap();
//
//         // Replace colors
//         let result = bitmap.map(|&color| {
//             if color == legacy_rule_frame_color {
//                 rule_frame_color
//             } else if color == legacy_rule_arrow_color {
//                 rule_arrow_color
//             } else {
//                 color
//             }
//         });
//
//         if result != bitmap {
//             println!("Saving {:?}", entry.path());
//             if let Err(_) = result.save(entry.path()) {
//                 println!("Failed to save {:?}", entry.path());
//             }
//         } else {
//             println!("{:?} not modified", entry.path());
//         }
//
//         println!("Processed file {:?}", entry.path());
//     }
// }

#[cfg(not(target_arch = "wasm32"))]
pub fn main_editor() {
    unsafe {
        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_drag_and_drop(true),
            #[cfg(feature = "force_120hz")]
            vsync: false,
            #[cfg(not(target_arch = "wasm32"))]
            window_builder: Some(Box::new(|builder| builder.with_maximized(true))),
            ..eframe::NativeOptions::default()
        };

        let result = eframe::run_native(
            "SeamLang",
            native_options,
            Box::new(|cc| {
                egui_extras::install_image_loaders(&cc.egui_ctx);
                use seamlang::app::EguiApp;
                let app = EguiApp::new(cc);
                Ok(Box::new(app))
            }),
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

        tracy_client::Client::start();

        // use seamlang::benchmarks::main_benchmark;
        // main_benchmark();

        // use seamlang::benchmarks::benchmark_run;
        // benchmark_run();

        // use seamlang::benchmarks::benchmark_topology_new;
        // benchmark_topology_new();

        main_editor();

        // color_replace();

        // main_benchmark_pixmap_regions();

        // main_benchmark_legacy_regions();
        // main_benchmark_pixmap_regions();
        // main_benchmark_field_regions();
    }
}
