use crate::math::rgba8::Rgba8;
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy)]
pub struct Material {
    rgba: Rgba8,
}

impl Material {
    pub const VOID: Self = Self::new(Rgba8::VOID);
    pub const RULE_FRAME: Self = Self::VOID;
    pub const TRANSPARENT: Self = Self::new(Rgba8::TRANSPARENT);
    pub const BLACK: Self = Self::new(Rgba8::BLACK);
    // const RULE_ARROW:

    pub const fn rgba(self) -> Rgba8 {
        self.rgba
    }

    pub const fn new(rgba: Rgba8) -> Self {
        Self { rgba }
    }

    pub fn is_rigid(&self) -> bool {
        self.rgba.a == 170
    }
}

impl PartialEq for Material {
    fn eq(&self, other: &Self) -> bool {
        // For rule frame and arrow (alpha = 180, 181) all rgb values are considered equal
        if self.rgba.a == 180 || self.rgba.a == 181 {
            self.rgba.a == other.rgba.a
        } else {
            self.rgba == other.rgba
        }
    }
}

impl Eq for Material {}

impl Ord for Material {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.rgba.a == 180 || self.rgba.a == 181 {
            self.rgba.a.cmp(&other.rgba.a)
        } else {
            self.rgba.cmp(&other.rgba)
        }
    }
}

impl PartialOrd for Material {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<Rgba8> for Material {
    fn from(rgba: Rgba8) -> Self {
        Material { rgba }
    }
}

impl From<Material> for Rgba8 {
    fn from(material: Material) -> Self {
        material.rgba
    }
}
