use crate::topology::ModificationTime;

#[derive(Clone, Copy, Debug)]
pub struct ValidDuration {
    /// Inclusive
    pub start: ModificationTime,

    /// Inclusive
    pub stop: ModificationTime,
}

impl ValidDuration {
    pub const INVALID: Self = Self {
        start: -1,
        stop: -2,
    };
}

/// Payload is valid between modified_atime and updated_atime
#[derive(Debug, Clone)]
pub struct Frozen<T> {
    pub payload: T,

    /// Payload is only valid during this duration. The stop time of the duration can be increased
    /// but never decreased.
    pub valid_duration: ValidDuration,
}

impl<T> Frozen<T> {
    pub fn update<S>(&mut self, arg: &Frozen<S>, f: impl FnOnce(&S) -> T) {
        if arg.valid_duration.start != self.valid_duration.start {
            self.payload = f(&arg.payload);
        }
        self.valid_duration = arg.valid_duration;
    }

    pub fn invalid(payload: T) -> Self {
        Self {
            payload,
            valid_duration: ValidDuration::INVALID,
        }
    }
}
