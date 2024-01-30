// #![allow(dead_code)]
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use seamlang::app::EguiApp;

pub fn main_benchmark() {
    use seamlang::{compiler::Compiler, rule::stabilize, topology::Topology};

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

#[cfg(not(target_arch = "wasm32"))]
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
    #[cfg(not(target_arch = "wasm32"))]
    main_editor()

    // main_benchmark();
}
