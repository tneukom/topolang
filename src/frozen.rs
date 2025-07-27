use crate::{
    math::pixel::Side,
    topology::{ModificationTime, Topology},
    world::solid_boundary_cycles,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct ValidDuration {
    /// Inclusive
    start: ModificationTime,

    /// Inclusive
    stop: ModificationTime,
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

pub type FrozenBoundary = Frozen<Arc<Vec<Vec<Side>>>>;

impl FrozenBoundary {
    pub fn update_boundary(&mut self, topology: &Topology) {
        let topology_modified_atime = topology.last_modification().map_or(0, |pair| pair.0);

        let solid_modified_atime = topology
            .modifications_after(self.valid_duration.stop)
            .filter_map(|(region_modified_atime, region_key)| {
                let region = &topology[region_key];
                region.material.is_solid().then_some(region_modified_atime)
            })
            .max();

        if let Some(solid_modified_atime) = solid_modified_atime {
            self.payload = Arc::new(solid_boundary_cycles(topology));
            self.valid_duration.start = solid_modified_atime;
        }
        self.valid_duration.stop = topology_modified_atime;
    }
}
