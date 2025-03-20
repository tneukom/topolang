use crate::{math::rgba8::Rgba8, utils::ReflectEnum};
use std::ops::{Range, RangeInclusive};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum MaterialClass {
    Normal,
    Solid,
    Rule,
    Wildcard,
    Transparent,
    Sleeping,
}

impl MaterialClass {
    pub const ALL: [Self; 6] = [
        Self::Normal,
        Self::Solid,
        Self::Rule,
        Self::Wildcard,
        Self::Transparent,
        Self::Sleeping,
    ];
}

impl ReflectEnum for MaterialClass {
    fn all() -> &'static [Self] {
        &Self::ALL
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Solid => "Solid",
            Self::Rule => "Rule",
            Self::Wildcard => "Wildcard",
            Self::Transparent => "Transparent",
            Self::Sleeping => "Sleeping",
        }
    }
}

// TODO: Material should be optimized for RegionEq, not converting from and to Rgba8!
// TODO: TRANSPARENT | SOLID should be possible, unclear how to represent as Rgba8 or on screen
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Material {
    pub rgb: [u8; 3],
    pub class: MaterialClass,
}

impl Material {
    pub const fn new(rgb: [u8; 3], class: MaterialClass) -> Self {
        Self { rgb, class }
    }

    // Opaque material (default)
    pub const OPAQUE_ALPHA: u8 = 255;

    // Solid materials
    #[deprecated]
    pub const LEGACY_SOLID_ALPHA: u8 = 170;

    pub const SOLID_MAIN_ALPHA: u8 = 254;
    pub const SOLID_LIGHTEN_ALPHA: u8 = 253;
    pub const SOLID_DARKEN_ALPHA_RANGE: Range<u8> =
        (Self::SOLID_LIGHTEN_ALPHA - 8)..Self::SOLID_LIGHTEN_ALPHA;
    pub const SOLID_ALPHA_RANGE: RangeInclusive<u8> =
        Self::SOLID_DARKEN_ALPHA_RANGE.start..=Self::SOLID_MAIN_ALPHA;

    // Rule materials
    #[deprecated]
    pub const LEGACY_RULE_ALPHA: u8 = 180;
    pub const LEGACY_RULE_ALPHA_2: u8 = 111;

    pub const RULE_GAP_ALPHA: u8 = 56;
    pub const RULE_BORDER_ALPHA: u8 = 191;
    pub const RULE_INTERIOR_ALPHA: u8 = 81;

    pub const RULE_ALPHAS: [u8; 5] = [
        Self::LEGACY_RULE_ALPHA,
        Self::LEGACY_RULE_ALPHA_2,
        Self::RULE_GAP_ALPHA,
        Self::RULE_BORDER_ALPHA,
        Self::RULE_INTERIOR_ALPHA,
    ];

    /// #360C29
    pub const RULE_BEFORE_RGB: [u8; 3] = [0x36, 0x0C, 0x29];

    pub const RULE_BEFORE: Self = Self::new(Self::RULE_BEFORE_RGB, MaterialClass::Rule);

    /// #FF6E00
    pub const RULE_AFTER_RGB: [u8; 3] = [0xFF, 0x6E, 0x00];

    pub const RULE_AFTER: Self = Self::new(Self::RULE_AFTER_RGB, MaterialClass::Rule);

    // Wildcard material
    pub const WILDCARD_ALPHA: u8 = 230;

    pub const WILDCARD_RAINBOW_RGB: [[u8; 3]; 6] = [
        [0xFF, 0x00, 0x00],
        [0xFF, 0x99, 0x00],
        [0xFF, 0xFF, 0x00],
        [0x33, 0xFF, 0x00],
        [0x00, 0x99, 0xFF],
        [0x66, 0x33, 0xFF],
    ];

    /// Use full rainbow instead of single color!
    #[deprecated]
    pub const WILDCARD: Self = Self::new(Self::WILDCARD_RAINBOW_RGB[0], MaterialClass::Wildcard);

    pub const SLEEPING_ALPHA: u8 = 131;
    pub const SLEEPING_ALT_ALPHA: u8 = 201;
    pub const SLEEPING_ALPHAS: [u8; 2] = [Self::SLEEPING_ALPHA, Self::SLEEPING_ALT_ALPHA];

    pub const UNDEF_COLOR: Rgba8 = Rgba8::new(0xFF, 0xFF, 0xFF, 0x00);

    // Some opaque color materials
    pub const BLACK: Self = Self::normal(Rgba8::BLACK.rgb());
    pub const RED: Self = Self::normal(Rgba8::RED.rgb());
    pub const GREEN: Self = Self::normal(Rgba8::GREEN.rgb());
    pub const BLUE: Self = Self::normal(Rgba8::BLUE.rgb());
    pub const YELLOW: Self = Self::normal(Rgba8::YELLOW.rgb());
    pub const CYAN: Self = Self::normal(Rgba8::CYAN.rgb());
    pub const MAGENTA: Self = Self::normal(Rgba8::MAGENTA.rgb());

    pub const TRANSPARENT: Self = Self::new(Rgba8::TRANSPARENT.rgb(), MaterialClass::Transparent);

    pub const fn normal(rgb: [u8; 3]) -> Self {
        Self {
            rgb,
            class: MaterialClass::Normal,
        }
    }

    pub fn is_solid(self) -> bool {
        self.class == MaterialClass::Solid
    }

    pub fn is_normal(self) -> bool {
        self.class == MaterialClass::Normal
    }

    pub fn is_rule(self) -> bool {
        self.class == MaterialClass::Rule
    }

    pub fn is_wildcard(self) -> bool {
        self.class == MaterialClass::Wildcard
    }

    pub fn is_sleeping(self) -> bool {
        self.class == MaterialClass::Sleeping
    }

    pub fn as_solid(mut self) -> Material {
        assert!(self.is_normal() || self.is_solid());
        self.class = MaterialClass::Solid;
        self
    }

    /// Discard flags like `Self::SOLID_FLAG`
    pub fn as_normal(mut self) -> Material {
        self.class = MaterialClass::Normal;
        self
    }

    /// Can a matching map a region with material `self` to a region with material `other`?
    /// Not symmetric!
    pub fn matches(self, other: Self) -> bool {
        if self.is_wildcard() {
            other != Self::TRANSPARENT
        } else {
            self == other
        }
    }

    pub fn solid_main_rgba(self) -> Rgba8 {
        Rgba8::from_rgb_a(self.rgb, Self::SOLID_MAIN_ALPHA)
    }

    pub fn solid_alt_rgba(self) -> Rgba8 {
        let [r, g, b] = self.rgb;

        if r >= 128 || g >= 128 || b >= 128 {
            // darken color
            let alpha_offset = (r & 1) | (g & 1) << 1 | (b & 1) << 2;
            let alpha = Self::SOLID_DARKEN_ALPHA_RANGE.start + alpha_offset;
            assert!(Self::SOLID_DARKEN_ALPHA_RANGE.contains(&alpha));
            let darkened_rgb = [r >> 1, g >> 1, b >> 1];
            Rgba8::from_rgb_a(darkened_rgb, alpha)
        } else {
            let lightened_rgb = [r << 1, g << 1, b << 1];
            Rgba8::from_rgb_a(lightened_rgb, Self::SOLID_LIGHTEN_ALPHA)
        }
    }

    /// Convert to Rgba8 without applying any effects
    pub fn to_rgba(self) -> Rgba8 {
        match self.class {
            MaterialClass::Normal => Rgba8::from_rgb_a(self.rgb, Self::OPAQUE_ALPHA),
            MaterialClass::Solid => Rgba8::from_rgb_a(self.rgb, Self::SOLID_MAIN_ALPHA),
            MaterialClass::Rule => Rgba8::from_rgb_a(self.rgb, Self::RULE_INTERIOR_ALPHA),
            MaterialClass::Wildcard => Rgba8::from_rgb_a(self.rgb, Self::WILDCARD_ALPHA),
            MaterialClass::Transparent => Rgba8::from_rgb_a(self.rgb, 0),
            MaterialClass::Sleeping => Rgba8::from_rgb_a(self.rgb, Self::SLEEPING_ALPHA),
        }
    }
}

impl From<Rgba8> for Material {
    /// Some materials have multiple variants that are considered equivalent. For example
    /// "rule before" and "rule after" can have different alpha values that are used to improve
    /// how the regions look by giving them a border effect.
    fn from(rgba: Rgba8) -> Self {
        let rgb = rgba.rgb();
        let [r, g, b, a] = rgba.to_array();

        if a == Self::OPAQUE_ALPHA {
            Self::new(rgb, MaterialClass::Normal)
        } else if a == 0 {
            Self::TRANSPARENT
        } else if Self::RULE_ALPHAS.contains(&a) {
            match rgb {
                Self::RULE_BEFORE_RGB => Self::RULE_BEFORE,
                Self::RULE_AFTER_RGB => Self::RULE_AFTER,
                _ => unimplemented!(),
            }
        } else if Self::SOLID_DARKEN_ALPHA_RANGE.contains(&a) {
            let alpha_offset = a - Self::SOLID_DARKEN_ALPHA_RANGE.start;
            let r_lsb = alpha_offset & 1;
            let g_lsb = (alpha_offset >> 1) & 1;
            let b_lsb = (alpha_offset >> 2) & 1;
            let rgb = [(r << 1) | r_lsb, (g << 1) | g_lsb, (b << 1) | b_lsb];
            Self::new(rgb, MaterialClass::Solid)
        } else if a == Self::SOLID_LIGHTEN_ALPHA {
            let rgb = [r >> 1, g >> 1, b >> 1];
            Self::new(rgb, MaterialClass::Solid)
        } else if a == Self::SOLID_MAIN_ALPHA {
            Self::new(rgb, MaterialClass::Solid)
        } else if a == Self::LEGACY_SOLID_ALPHA {
            Self::new(rgb, MaterialClass::Solid)
        } else if a == Self::WILDCARD_ALPHA {
            Self::new(rgb, MaterialClass::Wildcard)
        } else if Self::SLEEPING_ALPHAS.contains(&a) {
            Self::new(rgb, MaterialClass::Sleeping)
        } else {
            unimplemented!();
        }
    }
}

impl From<&Rgba8> for Material {
    fn from(rgba: &Rgba8) -> Self {
        Self::from(*rgba)
    }
}
