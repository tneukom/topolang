use crate::{field::RgbaField, pixmap::MaterialMap, world::World};

#[derive(Debug, Clone)]
pub struct Demo {
    pub filename: &'static str,
    pub png: &'static [u8],
}

impl Demo {
    pub const fn new(filename: &'static str, png: &'static [u8]) -> Self {
        Self { png, filename }
    }

    pub const PUZZLE_15: Demo = Demo::new(
        "15_puzzle.png",
        include_bytes!("../resources/saves/15_puzzle.png"),
    );

    pub const ADDER_4BIT: Demo = Demo::new(
        "4bit_adder.png",
        include_bytes!("../resources/saves/4bit_adder.png"),
    );

    pub const GAME_2048: Demo =
        Demo::new("2048.png", include_bytes!("../resources/saves/2048.png"));

    const FINITE_AUTOMATON: Demo = Demo::new(
        "automaton.png",
        include_bytes!("../resources/saves/automaton.png"),
    );

    pub const BINARY_COUNTER: Demo = Demo::new(
        "binary_counter.png",
        include_bytes!("../resources/saves/binary_counter.png"),
    );

    pub const RULE_30: Demo = Demo::new(
        "rule30.png",
        include_bytes!("../resources/saves/rule30.png"),
    );

    pub const RULE_110: Demo = Demo::new(
        "rule110.png",
        include_bytes!("../resources/saves/rule110.png"),
    );

    pub const DEMOS: [Demo; 7] = [
        Self::PUZZLE_15,
        Self::ADDER_4BIT,
        Self::GAME_2048,
        Self::FINITE_AUTOMATON,
        Self::BINARY_COUNTER,
        Self::RULE_30,
        Self::RULE_110,
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
