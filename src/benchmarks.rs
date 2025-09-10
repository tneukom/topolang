use crate::{
    compiler::Compiler, field::RgbaField, interpreter::Interpreter, pixmap::MaterialMap,
    rule::CanvasInput, topology::Topology, utils::IntoT, world::World,
};
use std::time::Instant;

pub fn benchmark_cellular_automaton() {
    let folder = "test_resources/benchmark";
    let mut world = World::load(format!("{folder}/cellular_automaton_triangles.png")).unwrap();

    let compiler = Compiler::new();
    let program = compiler.compile(&world).unwrap();
    let mut interpreter = Interpreter::new(program.clone());

    // First step takes much longer
    println!("First stabilize...");
    interpreter
        .tick(&mut world, &CanvasInput::default(), 10000)
        .ok()
        .unwrap();
    println!("Done");

    for _ in 0..500 {
        let now = Instant::now();
        let ticked = interpreter
            .tick(&mut world, &CanvasInput::default(), 10000)
            .ok()
            .unwrap();
        println!(
            "elapsed = {:.3?}, applications = {}",
            now.elapsed(),
            ticked.applications.len()
        );
    }
}

/// Run a scene by repeatedly stabilizing and waking up sleeping components.
pub fn benchmark_run() {
    let folder = "test_resources/benchmark";
    let world = World::load(format!("{folder}/hex_wave.png")).unwrap();

    let compiler = Compiler::new();
    let program = compiler.compile(&world).unwrap();

    // for (i_rule, rule) in rules.rules.iter().enumerate() {
    //     // Print regions in pattern
    //     for (region_key, region) in &rule.rule.before.topology.regions {
    //         println!(
    //             "Region {}: color = {:?}",
    //             region_key,
    //             region.material.to_rgba().hex()
    //         );
    //     }
    //
    //     let plan = &rule.rule.before.search_strategy.main_plan;
    //     println!("=== Plan {i_rule} ===");
    //     plan.print();
    // }

    for _ in 0..50 {
        let mut interpreter = Interpreter::new(program.clone());
        let mut world = world.clone();

        let now = Instant::now();
        let mut ticks = 0usize;
        loop {
            ticks += 1;
            let ticked = interpreter
                .tick(&mut world, &CanvasInput::default(), 1024)
                .unwrap();
            if !ticked.changed() {
                break;
            }

            // For debugging, save each image
            // world
            //     .material_map()
            //     .save(format!("benchmark_{ticks}.png"))
            //     .unwrap();
        }
        println!("elapsed = {:.3?}, ticks = {}", now.elapsed(), ticks);
    }
}

pub fn benchmark_compile() {
    let folder = "test_resources/benchmark";
    let world = World::load(format!("{folder}/generic_2048.png")).unwrap();

    let compiler = Compiler::new();
    for _ in 0..10 {
        let now = Instant::now();
        let program = compiler.compile(&world).unwrap();
        println!(
            "elapsed = {:.3?}, number of rule instances: {}",
            now.elapsed(),
            program.rule_instances_len()
        );
        // for rule in program.iter_rule_instances() {
        //     let steps_len = rule.rule.before.search_strategy.main_plan.steps.len();
        //     println!("steps_len: {steps_len}")
        // }
    }
}

pub fn benchmark_topology_new() {
    let folder = "test_resources/benchmark";
    let material_map = RgbaField::load(format!("{folder}/hex_wave.png"))
        .unwrap()
        .intot::<MaterialMap>();

    for _ in 0..100 {
        use std::time::Instant;
        let now = Instant::now();
        Topology::new(&material_map);
        println!("elapsed = {:.3?}", now.elapsed());
    }
}

pub fn benchmark_topology_draw() {
    let folder = "test_resources/benchmark";
    let before_material_map = MaterialMap::load(format!("{folder}/draw_before.png")).unwrap();
    let after_material_map = MaterialMap::load(format!("{folder}/draw_after.png")).unwrap();

    let changed_pixels: Vec<_> = before_material_map
        .field
        .indices()
        .filter_map(|pixel| {
            let from_material = before_material_map.get(pixel);
            let to_material = after_material_map.get(pixel);
            (from_material != to_material).then_some(pixel)
        })
        .collect();

    loop {
        tracy_client::frame_mark();

        let mut topology = Topology::new(&before_material_map);

        use std::time::Instant;
        let now = Instant::now();
        topology.update(&after_material_map, changed_pixels.iter().copied());
        println!("elapsed = {:.3?}", now.elapsed());
    }
}
