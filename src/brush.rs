use crate::math::rgba8::Rgba8;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Brush {
    pub color: Rgba8,
    pub radius: usize,
}

impl Brush {
    pub fn default() -> Self {
        Self {
            color: Rgba8::BLACK,
            radius: 1,
        }
    }
}
