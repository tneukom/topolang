use crate::{
    field::{Field, RgbaField},
    interpreter::Compiler,
    math::rgba8::Rgba8,
    pixmap::{MaterialMap, RgbaMap},
    regions::{field_regions_fast, pixmap_regions, CompactLabels},
    topology::Topology,
    utils::IntoT,
    world::World,
};
use std::time::{Duration, Instant};

pub fn main_benchmark_field_regions() {
    let folder = "test_resources/regions";
    let color_map = Field::load(format!("{folder}/b.png")).unwrap();

    let mut total_elapsed = Duration::from_millis(0);
    let mut compact_labels = CompactLabels::new(color_map.len());
    for _ in 0..1000 {
        let now = Instant::now();
        let _region_map = field_regions_fast(&color_map);
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
        let _region_map = pixmap_regions(&color_map);
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

/// Run a scene by repeatedly stabilizing and waking up sleeping components.
pub fn benchmark_run() {
    let folder = "test_resources/benchmark";
    let material_map = MaterialMap::load(format!("{folder}/hex_wave.png")).unwrap();
    let mut world = World::from(material_map);

    let compiler = Compiler::new();
    let rules = compiler.compile(&world).unwrap();

    for (i_rule, rule) in rules.rules.iter().enumerate() {
        // Print regions in pattern
        for (region_key, region) in &rule.rule.before.regions {
            println!(
                "Region {}: color = {:?}",
                region_key,
                region.material.to_rgba().hex()
            );
        }

        let plan = &rule.rule.search_plan;
        println!("=== Plan {i_rule} ===");
        plan.print();
    }

    let now = Instant::now();
    loop {
        let n_applications = rules.stabilize(&mut world, 1024);
        if n_applications == 0 {
            break;
        }
        rules.wake_up(&mut world);
        println!("Evolved n_applications = {n_applications}");
    }
    println!("elapsed = {:.3?}", now.elapsed());
}

pub fn main_benchmark() {
    let folder = "test_resources/benchmark";
    let original_world = RgbaField::load(format!("{folder}/gates4.png"))
        .unwrap()
        .intot::<MaterialMap>()
        .intot::<World>();

    let compiler = Compiler::new();

    for _ in 0..100 {
        use std::time::Instant;

        let mut world = original_world.clone();
        let compiled_rules = compiler.compile(&world).unwrap();

        let now = Instant::now();
        let mut steps = 0usize;
        while steps < 100 {
            steps += 1;
            let changed = compiled_rules.step(&mut world);
            if !changed {
                break;
            }
        }

        println!("steps = {}, elapsed = {:.3?}", steps, now.elapsed());
    }
}

pub fn benchmark_topology_new() {
    let folder = "resources/benchmarks";
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
