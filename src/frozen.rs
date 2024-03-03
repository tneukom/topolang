use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    ops::Deref,
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Debug, Clone, Copy, Eq)]
pub struct Frozen<T> {
    time: usize,
    payload: T,
    hash: u64,
}

pub fn counter() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

impl<T: Eq + Hash> Frozen<T> {
    pub fn new(payload: T) -> Self {
        let mut hasher = DefaultHasher::default();
        payload.hash(&mut hasher);
        Self {
            time: counter(),
            payload,
            hash: hasher.finish(),
        }
    }

    pub fn modify(frozen: &mut Self, f: impl FnOnce(&mut T)) {
        f(&mut frozen.payload);
        frozen.time = counter();
    }
}

impl<T: PartialEq> PartialEq for Frozen<T> {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time || self.payload == other.payload
    }
}

impl<T: Hash> Hash for Frozen<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

impl<T> Deref for Frozen<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}
