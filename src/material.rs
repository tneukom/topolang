use crate::{math::rgba8::Rgba8, regions::RegionEq};

// TODO: Material should be optimized for RegionEq, not converting from and to Rgba8!
// TODO: TRANSPARENT | SOLID should be possible, unclear how to represent as Rgba8 or on screen
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Material {
    rgb: [u8; 3],
    flags: u8,
}

impl Material {
    pub const SOLID_ALPHA: u8 = 170;
    pub const OPAQUE_ALPHA: u8 = 255;
    pub const RULE_ALPHA: u8 = 180;
    pub const TRANSPARENT_ALPHA: u8 = 0;

    pub const NO_FLAGS: u8 = 0b0000_0000;
    pub const SOLID_FLAG: u8 = 0b0000_0001;
    pub const TRANSPARENT_FLAG: u8 = 0b0000_0010;
    pub const RULE_FLAG: u8 = 0b0000_0100;

    /// Strips properties that are irrelevant to region equivalence relation
    pub const REGION_EQ_MASK: u8 = 0b1111_1110;

    /// Hex 360c29
    pub const VOID_COLOR: Rgba8 = Rgba8::new(0x36, 0x0C, 0x29, Self::RULE_ALPHA);
    pub const RULE_ARROW_COLOR: Rgba8 = Rgba8::new(0xFF, 0x6E, 0x00, Self::RULE_ALPHA);

    pub const VOID: Self = Self::from_rgba(Self::VOID_COLOR);
    pub const RULE_FRAME: Self = Self::VOID;
    pub const RULE_ARROW: Self = Self::from_rgba(Self::RULE_ARROW_COLOR);
    pub const TRANSPARENT: Self = Self::from_rgba(Rgba8::TRANSPARENT);
    pub const BLACK: Self = Self::from_rgba(Rgba8::BLACK);

    pub const fn new(rgb: [u8; 3], class: u8) -> Self {
        Self { rgb, flags: class }
    }

    pub const fn flags_to_alpha(flags: u8) -> u8 {
        match flags {
            Self::TRANSPARENT_FLAG => Self::TRANSPARENT_ALPHA,
            Self::SOLID_FLAG => Self::SOLID_ALPHA,
            Self::RULE_FLAG => Self::RULE_ALPHA,
            Self::NO_FLAGS => Self::OPAQUE_ALPHA,
            _ => unimplemented!(),
        }
    }

    pub const fn alpha_to_flags(alpha: u8) -> u8 {
        match alpha {
            Self::OPAQUE_ALPHA => Self::NO_FLAGS,
            Self::SOLID_ALPHA => Self::SOLID_FLAG,
            Self::RULE_ALPHA => Self::RULE_FLAG,
            Self::TRANSPARENT_ALPHA => Self::TRANSPARENT_FLAG,
            _ => unimplemented!(),
        }
    }

    pub const fn rgb(self) -> [u8; 3] {
        self.rgb
    }

    pub const fn from_rgba(rgba: Rgba8) -> Self {
        Self::new(rgba.rgb(), Self::alpha_to_flags(rgba.a))
    }

    pub const fn to_rgba(self) -> Rgba8 {
        Rgba8::from_rgb_a(self.rgb, Self::flags_to_alpha(self.flags))
    }

    pub fn is_rule(self) -> bool {
        self.flags & Self::RULE_FLAG != 0
    }

    pub fn is_solid(self) -> bool {
        self.flags & Self::SOLID_FLAG != 0
    }

    pub fn is_normal(self) -> bool {
        self.flags == Self::NO_FLAGS
    }

    pub fn as_solid(mut self) -> Material {
        assert!(self.is_normal());
        self.flags = Self::SOLID_FLAG;
        self
    }

    pub fn as_normal(mut self) -> Material {
        self.flags = Self::NO_FLAGS;
        self
    }
}

impl From<Rgba8> for Material {
    fn from(rgba: Rgba8) -> Self {
        Self::from_rgba(rgba)
    }
}

impl From<&Rgba8> for Material {
    fn from(rgba: &Rgba8) -> Self {
        Self::from_rgba(*rgba)
    }
}

impl From<Material> for Rgba8 {
    fn from(material: Material) -> Self {
        material.to_rgba()
    }
}

impl RegionEq for Material {
    fn region_eq(self, other: Self) -> bool {
        ((self.flags & Self::REGION_EQ_MASK) == (other.flags & Self::REGION_EQ_MASK))
            && (self.rgb == other.rgb)
    }
}
