use crate::{
    field::RgbaField,
    pixmap::MaterialMap,
    run_mode::{RunMode, RunSettings, RunSpeed},
    world::World,
};

#[derive(Debug, Clone)]
pub struct Demo {
    pub filename: &'static str,
    pub png: &'static [u8],
    pub autorun: RunSettings,
}

impl Demo {
    pub const fn new(filename: &'static str, png: &'static [u8], autorun: RunSettings) -> Self {
        Self {
            png,
            filename,
            autorun,
        }
    }

    pub const PUZZLE_15: Demo = Demo::new(
        "15_puzzle.png",
        include_bytes!("../resources/saves/15_puzzle.png"),
        RunSettings::new(RunMode::Run, RunSpeed::Hz30),
    );

    pub const GAME_2048: Demo = Demo::new(
        "2048.png",
        include_bytes!("../resources/saves/2048.png"),
        RunSettings::new(RunMode::Run, RunSpeed::Hz30),
    );

    pub const RULE_30: Demo = Demo::new(
        "rule30.png",
        include_bytes!("../resources/saves/rule30.png"),
        RunSettings::new(RunMode::Slowmo, RunSpeed::Hz30),
    );

    pub const RULE_110: Demo = Demo::new(
        "rule110.png",
        include_bytes!("../resources/saves/rule110.png"),
        RunSettings::new(RunMode::Slowmo, RunSpeed::Hz30),
    );

    pub const TRIANGLE_CELLULAR_AUTOMATON: Demo = Demo::new(
        "triangle_cellular_automaton.png",
        include_bytes!("../resources/saves/triangle_cellular_automaton.png"),
        RunSettings::new(RunMode::Run, RunSpeed::Hz2),
    );

    pub const GAME_OF_LIFE: Demo = Demo::new(
        "game_of_life.png",
        include_bytes!("../resources/saves/game_of_life.png"),
        RunSettings::new(RunMode::Slowmo, RunSpeed::Hz30),
    );

    pub const ADDER_4BIT: Demo = Demo::new(
        "4bit_adder.png",
        include_bytes!("../resources/saves/4bit_adder.png"),
        RunSettings::new(RunMode::Run, RunSpeed::Hz5),
    );

    const FINITE_AUTOMATON: Demo = Demo::new(
        "automaton.png",
        include_bytes!("../resources/saves/automaton.png"),
        RunSettings::new(RunMode::Slowmo, RunSpeed::Hz1),
    );

    pub const BINARY_COUNTER: Demo = Demo::new(
        "binary_counter.png",
        include_bytes!("../resources/saves/binary_counter.png"),
        RunSettings::new(RunMode::Run, RunSpeed::Hz30),
    );

    pub const TURING: Demo = Demo::new(
        "turing.png",
        include_bytes!("../resources/saves/turing.png"),
        RunSettings::new(RunMode::Slowmo, RunSpeed::Hz2),
    );

    pub const AUTUMN_TREE: Demo = Demo::new(
        "autumn_tree.png",
        include_bytes!("../resources/saves/autumn_tree.png"),
        RunSettings::new(RunMode::Run, RunSpeed::Hz30),
    );

    pub const TUTORIAL_BASICS: Demo = Demo::new(
        "tutorial_basics.png",
        include_bytes!("../tutorial/tutorial_basics.png"),
        RunSettings::new(RunMode::Paused, RunSpeed::Hz2),
    );

    pub const TUTORIAL_SLEEP: Demo = Demo::new(
        "tutorial_sleep.png",
        include_bytes!("../tutorial/tutorial_sleep.png"),
        RunSettings::new(RunMode::Paused, RunSpeed::Hz2),
    );

    pub const TUTORIAL_SOLID: Demo = Demo::new(
        "tutorial_solid.png",
        include_bytes!("../tutorial/tutorial_solid.png"),
        RunSettings::new(RunMode::Paused, RunSpeed::Hz2),
    );

    pub const HEX_WAVE: Demo = Demo::new(
        "hex_wave.png",
        include_bytes!("../resources/saves/hex_wave.png"),
        RunSettings::new(RunMode::Run, RunSpeed::Hz30),
    );

    pub const SIMPLE_TRAIN: Demo = Demo::new(
        "simple_train.png",
        include_bytes!("../resources/saves/simple_train.png"),
        RunSettings::new(RunMode::Run, RunSpeed::Hz5),
    );

    pub const DEMOS: [Demo; 13] = [
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
        Self::HEX_WAVE,
        Self::SIMPLE_TRAIN,
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

pub struct DemoSection {
    pub name: &'static str,
    pub demos: &'static [(&'static str, Demo)],
}

impl DemoSection {
    pub const GAMES: DemoSection = DemoSection {
        name: "Games",
        demos: &[("15 Puzzle", Demo::PUZZLE_15), ("2048", Demo::GAME_2048)],
    };

    pub const TUTORIALS: DemoSection = DemoSection {
        name: "Tutorials",
        demos: &[
            ("Basics", Demo::TUTORIAL_BASICS),
            ("Sleep", Demo::TUTORIAL_SLEEP),
            ("Solid", Demo::TUTORIAL_SOLID),
        ],
    };

    pub const CELLULAR_AUTOMATA: DemoSection = DemoSection {
        name: "Cellular Automata",
        demos: &[
            ("Rule 30", Demo::RULE_30),
            ("Rule 110", Demo::RULE_110),
            ("Triangles", Demo::TRIANGLE_CELLULAR_AUTOMATON),
            ("Game Of Life", Demo::GAME_OF_LIFE),
        ],
    };

    pub const COMPUTERS: DemoSection = DemoSection {
        name: "Computers",
        demos: &[
            ("4 Bit Adder", Demo::ADDER_4BIT),
            ("Finite Automaton", Demo::FINITE_AUTOMATON),
            ("Binary Counter", Demo::BINARY_COUNTER),
            ("Turing Machine", Demo::TURING),
        ],
    };

    pub const ANIMATION: DemoSection = DemoSection {
        name: "Animation",
        demos: &[
            ("Hex Wave", Demo::HEX_WAVE),
            ("Autumn Tree", Demo::AUTUMN_TREE),
        ],
    };

    pub const BASIC: DemoSection = DemoSection {
        name: "Basic",
        demos: &[("Simple Train", Demo::SIMPLE_TRAIN)],
    };

    pub const SECTIONS: [Self; 6] = [
        Self::TUTORIALS,
        Self::GAMES,
        Self::CELLULAR_AUTOMATA,
        Self::COMPUTERS,
        Self::ANIMATION,
        Self::BASIC,
    ];
}
