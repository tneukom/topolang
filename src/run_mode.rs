use crate::utils::ReflectEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RunSpeed {
    Hz1,
    Hz2,
    Hz5,
    Hz10,
    Hz30,
}

impl RunSpeed {
    const ALL: [Self; 5] = [Self::Hz1, Self::Hz2, Self::Hz5, Self::Hz10, Self::Hz30];

    pub fn frames_per_tick(self) -> usize {
        assert_eq!(60 % self.ticks_per_frame(), 0);
        60 / self.ticks_per_frame()
    }

    pub fn ticks_per_frame(self) -> usize {
        match self {
            Self::Hz1 => 1,
            Self::Hz2 => 2,
            Self::Hz5 => 5,
            Self::Hz10 => 10,
            Self::Hz30 => 30,
        }
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
