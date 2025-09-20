use crate::utils::ReflectEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RunSpeed {
    Hz1,
    Hz2,
    Hz5,
    Hz10,
    Hz30,
    Hz60,
}

impl RunSpeed {
    const ALL: [Self; 6] = [
        Self::Hz1,
        Self::Hz2,
        Self::Hz5,
        Self::Hz10,
        Self::Hz30,
        Self::Hz60,
    ];

    pub fn ticks_per_second(self) -> usize {
        match self {
            Self::Hz1 => 1,
            Self::Hz2 => 2,
            Self::Hz5 => 5,
            Self::Hz10 => 10,
            Self::Hz30 => 30,
            Self::Hz60 => 60,
        }
    }

    pub fn tick_dt(self) -> f64 {
        1.0 / self.ticks_per_second() as f64
    }
}

impl ReflectEnum for RunSpeed {
    fn all() -> &'static [Self] {
        &Self::ALL
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Hz1 => "1",
            Self::Hz2 => "2",
            Self::Hz5 => "5",
            Self::Hz10 => "10",
            Self::Hz30 => "30",
            Self::Hz60 => "60",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Paused,
    Slowmo,
    Run,
}

impl RunMode {
    pub const ALL: [Self; 3] = [Self::Paused, Self::Slowmo, Self::Run];
}

impl ReflectEnum for RunMode {
    fn all() -> &'static [Self] {
        &Self::ALL
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Paused => "Paused",
            Self::Slowmo => "Walk",
            Self::Run => "Run",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunSettings {
    pub mode: RunMode,
    pub speed: RunSpeed,
}

impl RunSettings {
    pub const fn new(mode: RunMode, speed: RunSpeed) -> Self {
        Self { mode, speed }
    }
}
