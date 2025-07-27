#![allow(dead_code)]
#![allow(unsafe_op_in_unsafe_fn)]

extern crate core;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

pub mod app;
pub mod benchmarks;
pub(crate) mod brush;
pub(crate) mod camera;
pub(crate) mod compiler;
pub(crate) mod coordinate_frame;
pub(crate) mod cycle_segments;
pub(crate) mod field;
pub(crate) mod gif_recorder;
pub(crate) mod history;
pub(crate) mod line_drawing;
pub(crate) mod material;
pub(crate) mod math;
pub(crate) mod morphism;
pub(crate) mod painting;
pub(crate) mod palettes;
pub(crate) mod pixmap;
pub(crate) mod regions;
pub(crate) mod rule;
pub(crate) mod topology;
pub(crate) mod utils;
pub(crate) mod view;
pub(crate) mod widgets;
pub(crate) mod world;

// pub(crate) mod material_effects;
pub(crate) mod frozen;
pub(crate) mod interpreter;
pub(crate) mod material_effects;
pub(crate) mod new_regions;
pub(crate) mod solver;

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
            let document = web_sys::window()
                .expect("No window")
                .document()
                .expect("No document");

            let canvas = document
                .get_element_by_id("the_canvas_id")
                .expect("Failed to find the_canvas_id")
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .expect("the_canvas_id was not a HtmlCanvasElement");

            eframe::WebRunner::new()
                .start(
                    canvas, // hardcode it
                    web_options,
                    Box::new(|cc| {
                        egui_extras::install_image_loaders(&cc.egui_ctx);
                        use crate::app::EguiApp;
                        let app = EguiApp::new(cc);
                        Ok(Box::new(app))
                    }),
                )
                .await
                .expect("failed to start eframe");
        });
    }
}
