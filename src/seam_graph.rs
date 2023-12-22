use crate::{
    math::rgba8::Rgba8,
    topology::{RegionKey, Topology},
};
use itertools::Itertools;
use std::{
    collections::{BTreeMap, BTreeSet},
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
    /// Seams are edges between regions, the right region can be empty space
    pub edges: BTreeSet<UndirectedEdge<RegionKey>>,

    /// Vertex labels
    pub region_colors: BTreeMap<RegionKey, Rgba8>,
}

impl SeamGraph {
    pub fn from_topology(topo: &Topology) -> SeamGraph {
        let mut edges = BTreeSet::new();

        for seam in topo.iter_seams() {
            let left_region = topo.left_of(seam);
            if let Some(right_region) = topo.right_of(seam) {
                let edge = UndirectedEdge::new(left_region, right_region);
                edges.insert(edge);
            }
        }

        let region_colors = topo
            .regions
            .iter()
            .map(|(&key, region)| (key, region.color))
            .collect();

        SeamGraph {
            edges,
            region_colors,
        }
    }

    /// Use colors for edges instead of the region keys.
    pub fn rgb_edges(&self) -> BTreeSet<UndirectedEdge<Rgba8>> {
        // Only works if all regions have different colors.
        assert!(self.region_colors.iter().all_unique());

        self.edges
            .iter()
            .map(|edge| {
                let a_rgb = self.region_colors[&edge.a];
                let b_rgb = self.region_colors[&edge.b];
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
            writeln!(f, "{:?}, {:?}", edge.a, edge.b)?;
        }

        writeln!(f, "Component colors")?;
        for (region_key, color) in self.region_colors.iter() {
            writeln!(f, "{:?}: {}", region_key, color)?;
        }

        Ok(())
    }
}
