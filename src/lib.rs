#![allow(dead_code)]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

pub(crate) mod brush;
pub(crate) mod camera;
pub(crate) mod coordinate_frame;
pub(crate) mod cycle_segments;
pub(crate) mod field;
pub(crate) mod history;
pub(crate) mod interpreter;
pub(crate) mod material;
pub(crate) mod math;
pub(crate) mod morphism;
pub(crate) mod painting;
pub(crate) mod palettes;
pub(crate) mod pattern;
pub(crate) mod pixmap;
pub(crate) mod regions;
pub(crate) mod rule;
pub(crate) mod topology;
pub(crate) mod union_find;
pub(crate) mod utils;
pub(crate) mod view;
pub(crate) mod widgets;
pub(crate) mod world;

pub mod app;
pub mod benchmarks;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn main() {
    use crate::app::EguiApp;
    use eframe::WebGlContextOption;
    use log::info;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let mut web_options = eframe::WebOptions::default();
    web_options.webgl_context_option = WebGlContextOption::WebGl2;

    unsafe {
        wasm_bindgen_futures::spawn_local(async {
            eframe::WebRunner::new()
                .start(
                    "the_canvas_id", // hardcode it
                    web_options,
                    Box::new(|cc| Box::new(EguiApp::new(cc))),
                )
                .await
                .expect("failed to start eframe");
        });
    }
}
