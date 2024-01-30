// #![allow(dead_code)]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

pub mod app;
pub mod array_2d;
pub mod bitmap;
pub mod brush;
pub mod camera;
pub mod compiler;
pub mod connected_components;
pub mod coordinate_frame;
pub mod math;
pub mod morphism;
pub mod painting;
pub mod palettes;
pub mod pattern;
pub mod pixmap;
pub mod rule;
pub mod topology;
pub mod utils;
pub mod view;
pub mod widgets;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn main() {
    use crate::app::EguiApp;
    use eframe::WebGlContextOption;

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
