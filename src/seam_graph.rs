use crate::{math::rgba8::Rgba8, topology::Topology};
use itertools::Itertools;
use std::{
    collections::BTreeSet,
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UndirectedEdge<T> {
    pub a: T,
    pub b: T,
}

impl<T: Ord> UndirectedEdge<T> {
    pub fn new(a: T, b: T) -> Self {
        if a < b {
            Self { a, b }
        } else {
            Self { a: b, b: a }
        }
    }
}

/// Useful for debugging, tests, visualization
/// Does not contain one sides Seams
#[derive(Debug, Clone)]
pub struct SeamGraph {
    /// Seams are edges between components, the right component can be empty space
    pub edges: BTreeSet<UndirectedEdge<usize>>,

    /// Vertex labels
    pub component_colors: Vec<Rgba8>,
}

impl SeamGraph {
    pub fn from_topology(topo: &Topology) -> SeamGraph {
        let mut edges = BTreeSet::new();
        for (seam, seam_indices) in &topo.seam_indices {
            let revered_seam = seam.reversed();
            if let Some(reversed_seam_indices) = topo.seam_indices.get(&revered_seam) {
                let edge = UndirectedEdge::new(
                    seam_indices.i_component,
                    reversed_seam_indices.i_component,
                );
                edges.insert(edge);
            }
        }

        let component_colors = topo.components.iter().map(|comp| comp.color).collect();

        SeamGraph {
            edges,
            component_colors,
        }
    }

    /// Use colors for edges instead of the component index.
    pub fn rgb_edges(&self) -> BTreeSet<UndirectedEdge<Rgba8>> {
        // Only works if all components are different
        assert!(self.component_colors.iter().all_unique());

        self.edges
            .iter()
            .map(|edge| {
                let a_rgb = self.component_colors[edge.a];
                let b_rgb = self.component_colors[edge.b];
                UndirectedEdge::new(a_rgb, b_rgb)
            })
            .collect()
    }
}

impl Display for SeamGraph {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // print graph (seams)
        writeln!(f, "Seams")?;
        for edge in &self.edges {
            writeln!(f, "{}, {}", edge.a, edge.b)?;
        }

        writeln!(f, "Component colors")?;
        for (i_component, color) in self.component_colors.iter().enumerate() {
            writeln!(f, "{}: {}", i_component, color)?;
        }

        Ok(())
    }
}
