use std::rc::Rc;

use crate::{pixmap::PixmapRgba, utils::unix_timestamp};

pub enum SnapshotCause {
    Root,
    Painting,
    Step,
    Run,
}

pub struct Snapshot {
    colormap: PixmapRgba,
    cause: SnapshotCause,

    /// Unix time
    timestamp: f64,

    parent: Option<Rc<Snapshot>>,
}

impl Snapshot {
    pub fn new(colormap: PixmapRgba, cause: SnapshotCause, parent: Option<Rc<Snapshot>>) -> Self {
        Self {
            colormap,
            timestamp: unix_timestamp(),
            cause,
            parent,
        }
    }

    // TODO: Could return an Iterator instead
    /// Resulting path contains start and stop
    pub fn path_to<'a>(
        mut self: &'a Rc<Snapshot>,
        stop: &'a Rc<Snapshot>,
    ) -> Option<Vec<&Rc<Snapshot>>> {
        let mut path = vec![self];
        while !Rc::ptr_eq(self, stop) {
            self = self.parent.as_ref()?;
            path.push(self);
        }
        Some(path)
    }

    pub fn colormap(&self) -> &PixmapRgba {
        &self.colormap
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
    pub fn new(colormap: PixmapRgba) -> Self {
        let root = Rc::new(Snapshot::new(colormap, SnapshotCause::Root, None));

        Self {
            head: root.clone(),
            active: root.clone(),
            root,
        }
    }

    /// If colormap is same as current no new snapshot is added
    pub fn add_snapshot(&mut self, colormap: PixmapRgba, cause: SnapshotCause) {
        if self.head.colormap() == &colormap {
            return;
        }
        self.head = Rc::new(Snapshot::new(colormap, cause, Some(self.head.clone())));
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

    /// If the user reverts to a snapshot in the history that snapshot is returned.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // List of snapshots with cause
        let path = self.active.path_to(&self.root).unwrap();
        egui::scroll_area::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(200.0)
            .show(ui, |ui| {
                for (i, snapshot) in path.into_iter().enumerate() {
                    let text = if Rc::ptr_eq(snapshot, &self.head) {
                        format!("> {i}")
                    } else {
                        format!("{i}")
                    };
                    if ui.button(text).clicked() {
                        // TODO: Set head to clicked snapshot
                        self.head = snapshot.clone();
                    }
                }
            });

        // Undo, Redo buttons
        if ui.button("Undo").clicked() {
            self.undo();
        }
        if ui.button("Redo").clicked() {
            self.redo();
        }
    }
}
