use std::rc::Rc;

use crate::{
    pixmap::Pixmap,
    utils::{unix_timestamp, ReflectEnum},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnapshotCause {
    Root,
    Brush,
    Erase,
    Fill,
    Step,
}

impl SnapshotCause {
    pub const ALL: [Self; 5] = [Self::Root, Self::Brush, Self::Erase, Self::Fill, Self::Step];
}

impl ReflectEnum for SnapshotCause {
    fn all() -> &'static [Self] {
        &Self::ALL
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Root => "Root",
            Self::Brush => "Brush",
            Self::Erase => "Erase",
            Self::Fill => "Fill",
            Self::Step => "Step",
        }
    }
}

pub struct Snapshot<M> {
    material_map: Pixmap<M>,
    cause: SnapshotCause,

    /// Unix time
    timestamp: f64,

    parent: Option<Rc<Snapshot<M>>>,
}

impl<M: Copy + Eq> Snapshot<M> {
    pub fn new(material_map: Pixmap<M>, cause: SnapshotCause, parent: Option<Rc<Self>>) -> Self {
        Self {
            material_map,
            timestamp: unix_timestamp(),
            cause,
            parent,
        }
    }

    // TODO: Could return an Iterator instead
    /// Resulting path contains start and stop
    pub fn path_to<'a>(mut self: &'a Rc<Self>, stop: &'a Rc<Self>) -> Option<Vec<&Rc<Self>>> {
        let mut path = vec![self];
        while !Rc::ptr_eq(self, stop) {
            self = self.parent.as_ref()?;
            path.push(self);
        }
        Some(path)
    }

    pub fn material_map(&self) -> &Pixmap<M> {
        &self.material_map
    }

    pub fn cause(&self) -> SnapshotCause {
        self.cause
    }
}

pub struct History<M> {
    /// Currently active node
    pub head: Rc<Snapshot<M>>,
    /// Leaf node
    pub active: Rc<Snapshot<M>>,
    pub root: Rc<Snapshot<M>>,
}

impl<M: Copy + Eq> History<M> {
    /// History should contain at least one item
    pub fn new(colormap: Pixmap<M>) -> Self {
        let root = Rc::new(Snapshot::new(colormap, SnapshotCause::Root, None));

        Self {
            head: root.clone(),
            active: root.clone(),
            root,
        }
    }

    /// If colormap is same as current no new snapshot is added
    pub fn add_snapshot(&mut self, material_map: Pixmap<M>, cause: SnapshotCause) {
        if self.head.material_map() == &material_map {
            return;
        }
        self.head = Rc::new(Snapshot::new(material_map, cause, Some(self.head.clone())));
        self.active = self.head.clone();
    }

    pub fn undo(&mut self) {
        if let Some(parent) = &self.head.parent {
            self.head = parent.clone();
        }
    }

    pub fn redo(&mut self) {
        // path starts at active and stops at head
        let path = self.active.path_to(&self.head).unwrap();
        if path.len() >= 2 {
            self.head = path[path.len() - 2].clone();
        }
    }
}
