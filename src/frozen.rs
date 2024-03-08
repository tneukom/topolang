use std::{
    cell::Cell,
    hash::{Hash, Hasher},
    ops::Deref,
};

#[derive(Debug, Clone, Copy, Eq)]
pub struct Frozen<T> {
    modified_time: usize,
    payload: T,
}

thread_local! {
    static COUNTER: Cell<usize> = const { Cell::new(0) };
}

pub fn counter() -> usize {
    // static COUNTER: AtomicUsize = AtomicUsize::new(1);
    // COUNTER.fetch_add(1, Ordering::Relaxed)

    COUNTER.with(|counter| {
        let value = counter.get() + 1;
        counter.set(value);
        value
    })
}

impl<T: Eq + Hash> Frozen<T> {
    pub fn new(payload: T) -> Self {
        Self {
            modified_time: counter(),
            payload,
        }
    }

    pub fn modify(frozen: &mut Self, f: impl FnOnce(&mut T)) {
        f(&mut frozen.payload);
        frozen.modified_time = counter();
    }

    pub fn modified_time(&self) -> usize {
        self.modified_time
    }
}

impl<T: PartialEq> PartialEq for Frozen<T> {
    fn eq(&self, other: &Self) -> bool {
        self.modified_time == other.modified_time || self.payload == other.payload
    }
}

impl<T: Hash> Hash for Frozen<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.payload.hash(state)
    }
}

impl<T> Deref for Frozen<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}
