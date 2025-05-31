use crate::{
    compiler::Compiler, field::RgbaField, interpreter::Interpreter, pixmap::MaterialMap,
    rule::CanvasInput, topology::Topology, utils::IntoT, world::World,
};
use std::time::Instant;

/// Run a scene by repeatedly stabilizing and waking up sleeping components.
pub fn benchmark_run() {
    let folder = "test_resources/benchmark";
    let material_map = MaterialMap::load(format!("{folder}/hex_wave.png")).unwrap();
    let world = World::from(material_map);

    let compiler = Compiler::new();
    let rules = compiler.compile(&world).unwrap();

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
        let mut interpreter = Interpreter::new(rules.clone());
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

pub fn main_benchmark() {
    let folder = "test_resources/benchmark";
    let original_world = RgbaField::load(format!("{folder}/gates.png"))
        .unwrap()
        .intot::<MaterialMap>()
        .intot::<World>();

    let compiler = Compiler::new();

    for _ in 0..100 {
        use std::time::Instant;

        let mut world = original_world.clone();
        let compiled_rules = compiler.compile(&world).unwrap();
        let mut interpreter = Interpreter::new(compiled_rules);

        let now = Instant::now();
        interpreter
            .stabilize(&mut world, &CanvasInput::default(), 100)
            .ok();

        println!("elapsed = {:.3?}", now.elapsed());
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
