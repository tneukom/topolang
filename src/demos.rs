use crate::{
    field::RgbaField,
    pixmap::MaterialMap,
    run_mode::{RunMode, RunSettings, RunSpeed},
    world::World,
};

#[derive(Debug, Clone)]
pub struct Demo {
    pub name: &'static str,
    pub filename: &'static str,
    pub png: &'static [u8],
    pub autorun: RunSettings,
}

impl Demo {
    pub const fn new(
        name: &'static str,
        filename: &'static str,
        png: &'static [u8],
        autorun: RunSettings,
    ) -> Self {
        Self {
            name,
            png,
            filename,
            autorun,
        }
    }

    pub const PUZZLE_15: Demo = Demo::new(
        "15 Puzzle",
        "15_puzzle.png",
        include_bytes!("../resources/saves/15_puzzle.png"),
        RunSettings::new(RunMode::Run, RunSpeed::Hz30),
    );

    pub const ADDER_4BIT: Demo = Demo::new(
        "4 Bit Adder",
        "4bit_adder.png",
        include_bytes!("../resources/saves/4bit_adder.png"),
        RunSettings::new(RunMode::Run, RunSpeed::Hz5),
    );

    pub const GAME_2048: Demo = Demo::new(
        "2048 Game",
        "2048.png",
        include_bytes!("../resources/saves/2048.png"),
        RunSettings::new(RunMode::Run, RunSpeed::Hz30),
    );

    const FINITE_AUTOMATON: Demo = Demo::new(
        "Finite Automaton",
        "automaton.png",
        include_bytes!("../resources/saves/automaton.png"),
        RunSettings::new(RunMode::Slowmo, RunSpeed::Hz1),
    );

    pub const BINARY_COUNTER: Demo = Demo::new(
        "Binary Counter",
        "binary_counter.png",
        include_bytes!("../resources/saves/binary_counter.png"),
        RunSettings::new(RunMode::Run, RunSpeed::Hz30),
    );

    pub const RULE_30: Demo = Demo::new(
        "Cellular Automaton - Rule 30",
        "rule30.png",
        include_bytes!("../resources/saves/rule30.png"),
        RunSettings::new(RunMode::Slowmo, RunSpeed::Hz30),
    );

    pub const RULE_110: Demo = Demo::new(
        "Cellular Automaton - Rule 110",
        "rule110.png",
        include_bytes!("../resources/saves/rule110.png"),
        RunSettings::new(RunMode::Slowmo, RunSpeed::Hz30),
    );

    pub const TURING: Demo = Demo::new(
        "Turing Machine",
        "turing.png",
        include_bytes!("../resources/saves/turing.png"),
        RunSettings::new(RunMode::Slowmo, RunSpeed::Hz2),
    );

    pub const GAME_OF_LIFE: Demo = Demo::new(
        "Game Of Life",
        "game_of_life.png",
        include_bytes!("../resources/saves/game_of_life.png"),
        RunSettings::new(RunMode::Slowmo, RunSpeed::Hz30),
    );

    pub const TRIANGLE_CELLULAR_AUTOMATON: Demo = Demo::new(
        "Triangle Cellular Automaton",
        "triangle_cellular_automaton.png",
        include_bytes!("../resources/saves/triangle_cellular_automaton.png"),
        RunSettings::new(RunMode::Run, RunSpeed::Hz2),
    );

    pub const AUTUMN_TREE: Demo = Demo::new(
        "Autumn Tree",
        "autumn_tree.png",
        include_bytes!("../resources/saves/autumn_tree.png"),
        RunSettings::new(RunMode::Run, RunSpeed::Hz30),
    );

    pub const DEMOS: [Demo; 11] = [
        Self::TURING,
        Self::PUZZLE_15,
        Self::GAME_2048,
        Self::ADDER_4BIT,
        Self::GAME_OF_LIFE,
        Self::FINITE_AUTOMATON,
        Self::BINARY_COUNTER,
        Self::RULE_30,
        Self::RULE_110,
        Self::TRIANGLE_CELLULAR_AUTOMATON,
        Self::AUTUMN_TREE,
    ];

    pub fn by_filename(filename: &str) -> Option<&'static Demo> {
        Self::DEMOS.iter().find(|demo| demo.filename == filename)
    }

    pub fn load_material_map(&self) -> MaterialMap {
        let rgba_field = RgbaField::load_from_memory(self.png).unwrap();
        rgba_field.into()
    }

    pub fn load_world(&self) -> World {
        World::from_material_map(self.load_material_map())
    }
}
