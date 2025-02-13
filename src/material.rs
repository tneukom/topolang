use crate::math::rgba8::Rgba8;

// TODO: Material should be optimized for RegionEq, not converting from and to Rgba8!
// TODO: TRANSPARENT | SOLID should be possible, unclear how to represent as Rgba8 or on screen
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Material {
    rgba: Rgba8,
}

impl Material {
    pub const SOLID_ALPHA: u8 = 170;
    pub const OPAQUE_ALPHA: u8 = 255;
    pub const RULE_ALPHA: u8 = 180;
    pub const TRANSPARENT_ALPHA: u8 = 0;

    /// #360c29
    pub const RULE_BEFORE_COLOR: Rgba8 = Rgba8::new(0x36, 0x0C, 0x29, Self::RULE_ALPHA);
    /// #FF6E00
    pub const RULE_AFTER_COLOR: Rgba8 = Rgba8::new(0xFF, 0x6E, 0x00, Self::RULE_ALPHA);
    /// #0C3619
    pub const WILDCARD_COLOR: Rgba8 = Rgba8::new(0x0C, 0x36, 0x19, Self::RULE_ALPHA);

    pub const UNDEF_COLOR: Rgba8 = Rgba8::new(0xFF, 0xFF, 0xFF, 0x00);

    pub const VOID: Self = Self::from_rgba(Self::RULE_BEFORE_COLOR);
    pub const RULE_BEFORE: Self = Self::VOID;
    pub const RULE_AFTER: Self = Self::from_rgba(Self::RULE_AFTER_COLOR);

    /// Wildcard matches any material except transparent
    pub const WILDCARD: Self = Self::from_rgba(Self::WILDCARD_COLOR);

    pub const TRANSPARENT: Self = Self::from_rgba(Rgba8::TRANSPARENT);
    pub const BLACK: Self = Self::from_rgba(Rgba8::BLACK);
    pub const RED: Self = Self::from_rgba(Rgba8::RED);
    pub const GREEN: Self = Self::from_rgba(Rgba8::GREEN);
    pub const BLUE: Self = Self::from_rgba(Rgba8::BLUE);

    pub const fn from_rgba(rgba: Rgba8) -> Self {
        Self { rgba }
    }

    pub const fn from_rgb_a(rgb: [u8; 3], a: u8) -> Self {
        Self {
            rgba: Rgba8::from_rgb_a(rgb, a),
        }
    }

    pub const fn to_rgba(self) -> Rgba8 {
        self.rgba
    }

    pub fn is_void(self) -> bool {
        self == Self::VOID
    }

    pub fn is_not_void(self) -> bool {
        !self.is_void()
    }

    pub fn is_solid(self) -> bool {
        self.rgba.a == Self::SOLID_ALPHA
    }

    pub fn is_normal(self) -> bool {
        self.rgba.a == Self::OPAQUE_ALPHA
    }

    pub fn as_solid(mut self) -> Material {
        assert!(self.is_normal() || self.is_solid());
        self.rgba.a = Self::SOLID_ALPHA;
        self
    }

    /// Discard flags like `Self::SOLID_FLAG`
    pub fn as_normal(mut self) -> Material {
        self.rgba.a = Self::OPAQUE_ALPHA;
        self
    }

    /// Can a matching map a region with material `self` to a region with material `other`?
    /// Not symmetric!
    pub fn matches(self, other: Self) -> bool {
        if self == Self::WILDCARD {
            other != Self::TRANSPARENT
        } else {
            self == other
        }
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
