use crate::{field::RgbaField, material::Material, math::rgba8::Rgba8, utils::ReflectEnum};
use itertools::Itertools;
use std::sync::OnceLock;

pub struct Palette {
    /// Non-empty set of colors
    pub colors: Vec<Rgba8>,

    pub name: String,
    pub link: String,
}

impl Palette {
    pub fn from_bitmap(
        bitmap: &RgbaField,
        name: impl Into<String>,
        link: impl Into<String>,
    ) -> Self {
        let colors: Vec<_> = bitmap.iter().copied().unique().collect();
        assert!(!colors.is_empty());

        Self {
            colors,
            name: name.into(),
            link: link.into(),
        }
    }

    pub fn palette_pico8() -> Palette {
        let bitmap = RgbaField::load_from_memory(include_bytes!("pico8.png")).unwrap();
        Self::from_bitmap(&bitmap, "PICO-8", "https://lospec.com/palette-list/pico-8")
    }

    pub fn palette_desatur8() -> Palette {
        let bitmap = RgbaField::load_from_memory(include_bytes!("desatur8.png")).unwrap();
        Self::from_bitmap(
            &bitmap,
            "DESATUR8",
            "https://lospec.com/palette-list/desatur8",
        )
    }

    pub fn palette_windows16() -> Palette {
        let bitmap = RgbaField::load_from_memory(include_bytes!("windows16.png")).unwrap();
        Self::from_bitmap(
            &bitmap,
            "WINDOWS-16",
            "https://lospec.com/palette-list/microsoft-vga",
        )
    }

    pub fn palette_na16() -> Palette {
        let bitmap = RgbaField::load_from_memory(include_bytes!("na16.png")).unwrap();
        Self::from_bitmap(&bitmap, "NA16", "https://lospec.com/palette-list/na16")
    }

    pub fn palette_hept32() -> Palette {
        let bitmap = RgbaField::load_from_memory(include_bytes!("hept32.png")).unwrap();
        Self::from_bitmap(&bitmap, "HEPT32", "https://lospec.com/palette-list/hept32")
    }

    pub fn palette_superfuture25() -> Palette {
        let bitmap = RgbaField::load_from_memory(include_bytes!("superfuture25.png")).unwrap();
        Self::from_bitmap(
            &bitmap,
            "SUPERFUTURE25",
            "https://lospec.com/palette-list/superfuture25",
        )
    }

    pub fn palettes() -> &'static [Palette] {
        static PALETTES: OnceLock<Vec<Palette>> = OnceLock::new();
        let palettes = PALETTES.get_or_init(|| {
            vec![
                Palette::palette_pico8(),
                Palette::palette_windows16(),
                Palette::palette_na16(),
                Palette::palette_desatur8(),
                Palette::palette_hept32(),
                Palette::palette_superfuture25(),
            ]
        });
        palettes.as_slice()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemPalette {
    RuleBefore,
    RuleAfter,
    Wildcard,
}

impl SystemPalette {
    pub const ALL: [Self; 3] = [Self::RuleBefore, Self::RuleAfter, Self::Wildcard];

    pub const fn rgba(self) -> Rgba8 {
        match self {
            Self::RuleBefore => Material::RULE_BEFORE.to_rgba(),
            Self::RuleAfter => Material::RULE_AFTER.to_rgba(),
            Self::Wildcard => Material::WILDCARD.to_rgba(),
        }
    }
}

impl ReflectEnum for SystemPalette {
    fn all() -> &'static [Self] {
        &Self::ALL
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::RuleBefore => "Rule Before",
            Self::RuleAfter => "Rule After",
            Self::Wildcard => "Wildcard",
        }
    }
}
