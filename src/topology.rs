use crate::{
    bitmap::Bitmap,
    connected_components::{color_components, pixelmap_from_bitmap, ColorComponent},
    math::{
        pixel::{Pixel, Side, Vertex},
        rgba8::Rgba8,
    },
};
use itertools::Itertools;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    fmt::{Display, Formatter},
    ops::Index,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};
use num_traits::abs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Seam {
    pub start: Side,
    pub stop: Side,
    pub len: usize
}

impl Seam {
    pub fn new_atom(start: Side, stop: Side) -> Self {
        Seam { start, stop, len: 1 }
    }

    pub fn new_with_len(start: Side, stop: Side, len: usize) -> Self {
        Seam { start, stop, len }
    }

    pub fn is_atom(&self) -> bool {
        self.len == 1
    }

    pub fn reversed(&self) -> Seam {
        assert!(self.is_atom());
        Self::new_atom(self.stop.reversed(), self.start.reversed())
    }

    pub fn start_corner(&self) -> Vertex {
        self.start.start_vertex()
    }

    pub fn stop_corner(&self) -> Vertex {
        self.stop.stop_vertex()
    }
}

impl Display for Seam {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Seam({} -> {})", self.start, self.stop)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeamColors {
    pub left: Rgba8,
    pub right: Rgba8,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct RegionKey {
    key: usize,
}

impl RegionKey {
    pub fn unused() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(1);
        Self {
            key: COUNTER.fetch_add(1, Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Border {
    pub cycle: Vec<Side>,
    pub seams: Vec<Seam>,
    pub left: RegionKey,
}

/// The boundary is counter clockwise. Is mutable so color can be changed.
#[derive(Debug, Clone)]
pub struct Region {
    pub color: Rgba8,
    pub boundary: Vec<Border>,

    pub interior: BTreeSet<Pixel>,

    /// If the region is not closed left_of(boundary) is not defined otherwise
    /// left_of(boundary) = interior
    pub closed: bool,
}

impl Region {
    pub fn iter_seams(&self) -> impl Iterator<Item = &Seam> {
        self.boundary.iter().flat_map(|border| border.seams.iter())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeamIndex {
    pub region_key: RegionKey,
    pub i_border: usize,
    pub i_seam: usize,
}

impl SeamIndex {
    pub fn new(region_key: RegionKey, i_border: usize, i_seam: usize) -> Self {
        Self {
            region_key,
            i_border,
            i_seam,
        }
    }

    pub fn border_index(&self) -> BorderKey {
        BorderKey {
            region_key: self.region_key,
            i_border: self.i_border,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BorderKey {
    pub region_key: RegionKey,
    pub i_border: usize,
}

impl BorderKey {
    pub fn new(region_key: RegionKey, i_border: usize) -> Self {
        Self {
            region_key,
            i_border,
        }
    }
}

pub struct Topology {
    pub regions: BTreeMap<RegionKey, Region>,
    /// Maps the start side of a seam to (region key, border index, seam index)
    pub seam_indices: BTreeMap<Side, SeamIndex>,
}

impl Topology {
    pub fn new(pixelmap: &BTreeMap<Pixel, Rgba8>) -> Self {
        let connected_components = color_components(pixelmap);

        let mut regions: BTreeMap<RegionKey, Region> = BTreeMap::new();
        let mut seam_indices: BTreeMap<Side, SeamIndex> = BTreeMap::new();

        for ColorComponent { component, color } in connected_components {
            let region_key = RegionKey::unused();
            // Each cycle in the sides is a border
            let mut boundary = Vec::new();

            for cycle in split_into_cycles(&component.sides) {
                let i_border = boundary.len();
                let seams =
                    split_cycle_into_seams(&cycle, |side| pixelmap.get(&side.right_pixel()));

                for (i_seam, &seam) in seams.iter().enumerate() {
                    let seam_index = SeamIndex::new(region_key, i_border, i_seam);
                    seam_indices.insert(seam.start, seam_index);
                }

                let border = Border {
                    cycle,
                    seams,
                    left: region_key,
                };
                boundary.push(border);
            }

            let region = Region {
                boundary,
                color,
                interior: component.interior,
                closed: component.closed,
            };
            regions.insert(region_key, region);
        }

        Topology {
            regions,
            seam_indices,
        }
    }

    // pub fn remove_region(&mut self, key: RegionKey) -> Option<Region> {
    //     if let Some(region) = self.regions.remove(&key) {
    //         for seam in region.iter_seams() {
    //             self.seam_indices.remove(seam)
    //         }
    //         Some(region)
    //     } else {
    //         None
    //     }
    // }
    //
    // /// Undefined behaviour if region overlaps existing regions
    // pub fn insert_region(&mut self, key: RegionKey, region: Region) {
    //     assert!(!self.regions.contains_key(&key));
    //
    //     for (i_border, border) in region.boundary.iter().enumerate() {
    //         for (i_seam, seam) in border.seams.iter().enumerate() {
    //             assert!(!self.seam_indices.contains_key(seam));
    //             let seam_index = SeamIndex::new(key, i_border, i_seam);
    //             self.seam_indices.insert(*seam, seam_index);
    //         }
    //     }
    //
    //     let previous_entry = self.regions.insert(key, region);
    //     assert!(previous_entry.is_none());
    // }

    pub fn iter_borders(&self) -> impl Iterator<Item = &Border> + Clone {
        self.regions.values().flat_map(|region| region.boundary.iter())
    }

    pub fn iter_seams(&self) -> impl Iterator<Item = &Seam> + Clone {
        self.iter_borders().flat_map(|border| border.seams.iter())
    }

    pub fn iter_seam_indices(&self) -> impl Iterator<Item = &SeamIndex> {
        self.seam_indices.values()
    }

    pub fn iter_region_keys<'a>(&'a self) -> impl Iterator<Item = &RegionKey> + Clone + 'a {
        self.regions.keys()
    }

    pub fn iter_region_values<'a>(&'a self) -> impl Iterator<Item = &Region> + Clone + 'a {
        self.regions.values()
    }

    pub fn from_bitmap_path(path: impl AsRef<Path>) -> anyhow::Result<Topology> {
        let bitmap = Bitmap::from_path(path)?;
        let pixelmap = pixelmap_from_bitmap(&bitmap);
        Ok(Topology::new(&pixelmap))
    }

    pub fn contains_seam(&self, seam: &Seam) -> bool {
        self.seam_indices.contains_key(&seam.start)
    }

    /// Returns the border of a given seam
    /// Only fails if seam is not self.contains_seam(seam)
    pub fn seam_border(&self, seam: &Seam) -> &Border {
        let seam_index = &self.seam_indices[&seam.start];
        &self.regions[&seam_index.region_key].boundary[seam_index.i_border]
    }

    /// Return the region key on the left side of a given seam
    /// Only fails if seam is not self.contains_seam(seam)
    pub fn left_of(&self, seam: &Seam) -> RegionKey {
        let seam_index = &self.seam_indices[&seam.start];
        seam_index.region_key
    }

    /// Not every seam has a region on the right, it can be empty space
    pub fn right_of(&self, seam: &Seam) -> Option<RegionKey> {
        assert!(seam.is_atom());
        let seam_index = self.seam_indices.get(&seam.reversed().start)?;
        Some(seam_index.region_key)
    }

    /// Only fails if seam is not self.contains_seam(seam)
    pub fn next_seam(&self, seam: &Seam) -> &Seam {
        let seam_index = self.seam_indices[&seam.start];
        let border = &self.regions[&seam_index.region_key].boundary[seam_index.i_border];
        &border.seams[(seam_index.i_seam + seam.len) % border.seams.len()]
    }

    /// Only fails if seam is not self.contains_seam(seam)
    pub fn previous_seam(&self, seam: &Seam) -> &Seam {
        let seam_index = self.seam_indices[&seam.start];
        let border = &self.regions[&seam_index.region_key].boundary[seam_index.i_border];
        &border.seams[(seam_index.i_seam - 1 + border.seams.len()) % border.seams.len()]
    }

    pub fn seam_colors(&self, seam: &Seam) -> SeamColors {
        SeamColors {
            left: self[self.left_of(seam)].color,
            right: self
                .right_of(seam)
                .map_or(Rgba8::TRANSPARENT, |right| self[right].color),
        }
    }

    /// Seam between left and right region
    /// Component index errors cause panic
    pub fn seams_between(&self, left: RegionKey, right: RegionKey) -> impl Iterator<Item = &Seam> {
        let left_comp = &self.regions[&left];
        left_comp
            .iter_seams()
            .filter(move |&seam| self.right_of(seam) == Some(right))
    }

    /// Is the right side of the seam void? The left side is always a proper region.
    pub fn touches_void(&self, seam: &Seam) -> bool {
        self.right_of(seam).is_none()
    }

    // pub fn connected_borders(&self, seed: BorderKey) -> BTreeSet<BorderKey> {
    //     let mut visited: BTreeSet<BorderKey> = BTreeSet::new();
    //     let mut todos: Vec<BorderKey> = vec![seed];
    //
    //     while let Some(todo) = todos.pop() {
    //         if visited.insert(todo) {
    //             // newly inserted
    //             for seam in &self[seed].seams {
    //                 if let Some(reversed_seam_index) = self.seam_indices.get(&seam.reversed().start) {
    //                     todos.push(reversed_seam_index.border_index());
    //                 }
    //             }
    //         }
    //     }
    //
    //     visited
    // }

    // /// Moves everything on the left side of `border_key` to `into` including `border_key`
    // /// itself.
    // pub fn extract_left_of_border(&mut self, border_key: BorderKey, into: &mut Topology) {
    //     let region_key = border_key.region_key;
    //     let region = self.remove_region(region_key).unwrap();
    //
    //     for (i_other, other_border) in region.boundary.iter().enumerate() {
    //         if i_other != border_key.i_border {
    //             self.extract_right_of_border(BorderKey::new(border_key.region_key, i_other), into);
    //         }
    //     }
    //
    //     into.insert_region(region_key, region);
    // }
    //
    // /// Moves everything on the right side of `border_key` to `into`, NOT including
    // /// the boundary itself.
    // pub fn extract_right_of_border(&mut self, border_key: BorderKey, into: &mut Topology) {
    //     for other_border_key in self.connected_borders(border_key) {
    //         if other_border_key != border_key {
    //             self.extract_left_of_border(other_border_key, into);
    //         }
    //     }
    // }
}

impl Index<RegionKey> for Topology {
    type Output = Region;

    fn index(&self, key: RegionKey) -> &Self::Output {
        &self.regions[&key]
    }
}

impl Index<BorderKey> for Topology {
    type Output = Border;

    fn index(&self, key: BorderKey) -> &Self::Output {
        &self.regions[&key.region_key].boundary[key.i_border]
    }
}

impl Display for Topology {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let indent = "    ";

        for (region_key, region) in self.regions.iter() {
            writeln!(f, "Component {region_key:?}")?;
            for (i_border, border) in region.boundary.iter().enumerate() {
                writeln!(f, "{indent}Border {i_border}")?;
                for seam in &border.seams {
                    writeln!(f, "{indent}{indent}{seam}")?;
                }
            }
        }

        Ok(())
    }
}

/// Extract the side cycle that starts (and stops) at start_corner
pub fn extract_cycle(side_graph: &mut BTreeMap<Vertex, Side>, mut corner: Vertex) -> Vec<Side> {
    let mut cycle = Vec::new();
    while let Some(side) = side_graph.remove(&corner) {
        cycle.push(side);
        corner = side.stop_vertex();
    }
    assert_eq!(
        cycle.first().unwrap().start_vertex(),
        cycle.last().unwrap().stop_vertex(),
        "borders with gaps are not allowed"
    );
    cycle
}

/// Split a set of sides into cycles. The start point of every cycle is its minimum
/// (lexicographic order x, y). `sides` must be an iterable of Side.
pub fn split_into_cycles<'a>(sides: impl IntoIterator<Item = &'a Side>) -> Vec<Vec<Side>> {
    // Maps corner to side that starts at the corner
    let mut side_graph: BTreeMap<Vertex, Side> = sides
        .into_iter()
        .map(|&side| (side.start_vertex(), side))
        .collect();

    let mut cycles = Vec::new();

    while let Some((&corner, _)) = side_graph.first_key_value() {
        let cycle = extract_cycle(&mut side_graph, corner);
        cycles.push(cycle);
    }

    cycles
}

/// Split a side cycle into segments based on a function f: Side -> T
/// Each segment has constant f
pub fn split_cycle_into_seams<T: Eq>(cycle: &Vec<Side>, f: impl Fn(Side) -> T) -> Vec<Seam> {
    let discontinuities: Vec<_> = cycle
        .iter()
        .circular_tuple_windows()
        .filter(|(&lhs, &rhs)| f(lhs) != f(rhs))
        .collect();

    if discontinuities.is_empty() {
        // TODO: Reverse cycle should give same seam
        vec![Seam::new_atom(*cycle.first().unwrap(), *cycle.last().unwrap())]
    } else {
        discontinuities
            .iter()
            .circular_tuple_windows()
            .map(|((_, &lhs), (&rhs, _))| Seam::new_atom(lhs, rhs))
            .collect()
    }
}

#[cfg(test)]
mod test {
    use crate::{
        bitmap::Bitmap,
        connected_components::pixelmap_from_bitmap,
        math::rgba8::Rgba8,
        seam_graph::{SeamGraph, UndirectedEdge},
        topology::Topology,
    };
    use std::collections::BTreeSet;

    fn load_topology(filename: &str) -> Topology {
        let path = format!("test_resources/topology/{filename}");
        let bitmap = Bitmap::from_path(path).unwrap();
        let pixelmap = pixelmap_from_bitmap(&bitmap);
        let topology = Topology::new(&pixelmap);
        topology
    }

    fn load_rgb_seam_graph(filename: &str) -> BTreeSet<UndirectedEdge<Rgba8>> {
        let topo = load_topology(filename);
        let seam_graph = SeamGraph::from_topology(&topo);
        seam_graph.rgb_edges()
    }

    fn rgb_edges_from<const N: usize>(
        rgb_edges: [(Rgba8, Rgba8); N],
    ) -> BTreeSet<UndirectedEdge<Rgba8>> {
        rgb_edges
            .into_iter()
            .map(|(a, b)| UndirectedEdge::new(a, b))
            .collect()
    }

    #[test]
    fn image_1a() {
        let rgb_edges = load_rgb_seam_graph("1a.png");
        let expected_rgb_edges = rgb_edges_from([
            (Rgba8::RED, Rgba8::TRANSPARENT),
        ]);
        assert_eq!(rgb_edges, expected_rgb_edges);
    }

    #[test]
    fn image_2a() {
        let rgb_edges = load_rgb_seam_graph("2a.png");
        let expected_rgb_edges = rgb_edges_from([
            (Rgba8::RED, Rgba8::BLUE),
            (Rgba8::RED, Rgba8::TRANSPARENT),
            (Rgba8::BLUE, Rgba8::TRANSPARENT),
        ]);
        assert_eq!(rgb_edges, expected_rgb_edges);
    }

    #[test]
    fn image_2b() {
        let rgb_edges = load_rgb_seam_graph("2b.png");
        let expected_rgb_edges =
            rgb_edges_from([(Rgba8::RED, Rgba8::BLUE), (Rgba8::RED, Rgba8::TRANSPARENT)]);
        assert_eq!(rgb_edges, expected_rgb_edges);
    }

    #[test]
    fn image_3a() {
        let rgb_edges = load_rgb_seam_graph("3a.png");
        let expected_rgb_edges = rgb_edges_from([
            (Rgba8::RED, Rgba8::GREEN),
            (Rgba8::RED, Rgba8::BLUE),
            (Rgba8::RED, Rgba8::TRANSPARENT),
        ]);
        assert_eq!(rgb_edges, expected_rgb_edges);
    }

    #[test]
    fn image_3b() {
        let rgb_edges = load_rgb_seam_graph("3b.png");
        let expected_rgb_edges =
            rgb_edges_from([(Rgba8::RED, Rgba8::GREEN), (Rgba8::GREEN, Rgba8::BLUE)]);
        assert_eq!(rgb_edges, expected_rgb_edges);
    }

    #[test]
    fn image_3c() {
        let rgb_edges = load_rgb_seam_graph("3c.png");
        let expected_rgb_edges = rgb_edges_from([
            (Rgba8::RED, Rgba8::YELLOW),
            (Rgba8::RED, Rgba8::BLUE),
            (Rgba8::YELLOW, Rgba8::BLUE),
            (Rgba8::RED, Rgba8::TRANSPARENT),
        ]);
        assert_eq!(rgb_edges, expected_rgb_edges);
    }

    #[test]
    fn image_4a() {
        let rgb_edges = load_rgb_seam_graph("4a.png");
        let expected_rgb_edges = rgb_edges_from([
            (Rgba8::RED, Rgba8::BLUE),
            (Rgba8::BLUE, Rgba8::GREEN),
            (Rgba8::GREEN, Rgba8::CYAN),
            (Rgba8::BLUE, Rgba8::CYAN),
            (Rgba8::RED, Rgba8::CYAN),
            (Rgba8::RED, Rgba8::TRANSPARENT),
        ]);
        assert_eq!(rgb_edges, expected_rgb_edges);
    }
}
