use crate::{pixmap::MaterialMap, utils::ReflectEnum, view::Selection};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnapshotCause {
    Root,
    Brush,
    Erase,
    Fill,
    Tick,
    Run,
    Resized,
    Selected,
    SelectionCancelled,
    SelectionMoved,
}

impl SnapshotCause {
    pub const ALL: [Self; 10] = [
        Self::Root,
        Self::Brush,
        Self::Erase,
        Self::Fill,
        Self::Tick,
        Self::Run,
        Self::Resized,
        Self::Selected,
        Self::SelectionCancelled,
        Self::SelectionMoved,
    ];
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
            Self::Tick => "Tick",
            Self::Run => "Run",
            Self::Resized => "Resized",
            Self::Selected => "Selected",
            Self::SelectionCancelled => "Selection cancelled",
            Self::SelectionMoved => "Selection moved",
        }
    }
}

pub struct Snapshot {
    material_map: MaterialMap,
    selection: Option<Selection>,
    cause: SnapshotCause,

    parent: Option<Rc<Snapshot>>,
}

impl Snapshot {
    pub fn new(
        material_map: MaterialMap,
        selection: Option<Selection>,
        cause: SnapshotCause,
        parent: Option<Rc<Self>>,
    ) -> Self {
        Self {
            material_map,
            selection,
            cause,
            parent,
        }
    }

    // TODO: Could return an Iterator instead
    /// Resulting path contains start and stop
    pub fn path_to<'a>(mut self: &Rc<Self>, stop: &Rc<Self>) -> Option<Vec<&Rc<Self>>> {
        let mut path = vec![self];
        while !Rc::ptr_eq(self, stop) {
            self = self.parent.as_ref()?;
            path.push(self);
        }
        Some(path)
    }

    pub fn material_map(&self) -> &MaterialMap {
        &self.material_map
    }

    pub fn selection(&self) -> &Option<Selection> {
        &self.selection
    }

    pub fn cause(&self) -> SnapshotCause {
        self.cause
    }
}

pub struct History {
    /// Currently active node
    pub head: Rc<Snapshot>,
    /// Leaf node
    pub active: Rc<Snapshot>,
    pub root: Rc<Snapshot>,
}

impl History {
    /// History should contain at least one item
    pub fn new(colormap: MaterialMap, selection: Option<Selection>) -> Self {
        let root = Rc::new(Snapshot::new(
            colormap,
            selection,
            SnapshotCause::Root,
            None,
        ));

        Self {
            head: root.clone(),
            active: root.clone(),
            root,
        }
    }

    /// If colormap is same as current no new snapshot is added
    pub fn add_snapshot(
        &mut self,
        material_map: MaterialMap,
        selection: Option<Selection>,
        cause: SnapshotCause,
    ) {
        if self.head.material_map() == &material_map && self.head.selection == selection {
            println!("New snapshot with no changes!");
            return;
        }
        self.head = Rc::new(Snapshot::new(
            material_map,
            selection,
            cause,
            Some(self.head.clone()),
        ));
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
