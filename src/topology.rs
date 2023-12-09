use crate::{
    bitmap::Bitmap,
    connected_components::{connected_components, pixelmap_from_bitmap},
    math::{
        pixel::{Pixel, Side, Vertex},
        rgba8::Rgba8,
    },
};
use itertools::Itertools;
use std::{
    collections::BTreeMap,
    fmt,
    fmt::{Display, Formatter},
    ops::Index,
    path::Path,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Seam {
    pub start: Side,
    pub stop: Side,
}

impl Seam {
    pub fn new(start: Side, stop: Side) -> Self {
        Seam { start, stop }
    }

    pub fn reversed(&self) -> Seam {
        Self::new(self.stop.reversed(), self.start.reversed())
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

pub type RegionKey = usize;

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
    pub fn new(region_key: usize, i_border: usize, i_seam: usize) -> Self {
        Self {
            region_key,
            i_border,
            i_seam,
        }
    }
}

pub struct Topology {
    pub regions: Vec<Region>,
    /// Maps seams to (region key, border index, seam index)
    pub seam_indices: BTreeMap<Seam, SeamIndex>,
}

impl Topology {
    pub fn new(pixelmap: &BTreeMap<Pixel, Rgba8>) -> Self {
        let connected_components = connected_components(pixelmap);

        let mut regions = Vec::new();
        let mut seam_indices: BTreeMap<Seam, SeamIndex> = BTreeMap::new();

        for connected_component in &connected_components {
            // Each cycle in the sides is a border
            let mut boundary = Vec::new();
            let region_key = regions.len();

            for cycle in split_into_cycles(&connected_component.sides) {
                let i_border = boundary.len();
                let seams =
                    split_cycle_into_seams(&cycle, |side| pixelmap.get(&side.right_pixel()));

                for (i_seam, &seam) in seams.iter().enumerate() {
                    let seam_index = SeamIndex::new(region_key, i_border, i_seam);
                    seam_indices.insert(seam, seam_index);
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
                color: connected_component.color,
            };
            regions.push(region);
        }

        Topology {
            regions,
            seam_indices,
        }
    }

    pub fn iter_seams(&self) -> impl Iterator<Item = &Seam> + Clone {
        self.seam_indices.keys()
    }

    pub fn iter_regions(&self) -> impl Iterator<Item = RegionKey> + Clone {
        0..self.regions.len()
    }

    pub fn from_bitmap_path(path: impl AsRef<Path>) -> anyhow::Result<Topology> {
        let bitmap = Bitmap::from_path(path)?;
        let pixelmap = pixelmap_from_bitmap(&bitmap);
        Ok(Topology::new(&pixelmap))
    }

    pub fn contains_seam(&self, seam: &Seam) -> bool {
        self.seam_indices.contains_key(seam)
    }

    /// Returns the border of a given seam
    /// Only fails if seam is not self.contains_seam(seam)
    pub fn seam_border(&self, seam: &Seam) -> &Border {
        let seam_index = self.seam_indices[seam];
        &self.regions[seam_index.region_key].boundary[seam_index.i_border]
    }

    /// Return the region key on the left side of a given seam
    /// Only fails if seam is not self.contains_seam(seam)
    pub fn left_of(&self, seam: &Seam) -> RegionKey {
        let seam_index = &self.seam_indices[seam];
        seam_index.region_key
    }

    /// Not every seam has a region on the right, it can be empty space
    pub fn right_of(&self, seam: &Seam) -> Option<RegionKey> {
        let seam_index = self.seam_indices.get(&seam.reversed())?;
        Some(seam_index.region_key)
    }

    /// Only fails if seam is not self.contains_seam(seam)
    pub fn next_seam(&self, seam: &Seam) -> &Seam {
        let seam_index = self.seam_indices[seam];
        let border = &self.regions[seam_index.region_key].boundary[seam_index.i_border];
        &border.seams[(seam_index.i_seam + 1) % border.seams.len()]
    }

    /// Only fails if seam is not self.contains_seam(seam)
    pub fn previous_seam(&self, seam: &Seam) -> &Seam {
        let seam_index = self.seam_indices[seam];
        let border = &self.regions[seam_index.region_key].boundary[seam_index.i_border];
        &border.seams[(seam_index.i_seam - 1) % border.seams.len()]
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
        let left_comp = &self.regions[left];
        left_comp
            .iter_seams()
            .filter(move |&seam| self.right_of(seam) == Some(right))
    }

    /// Is the right side of the seam void? The left side is always a proper region.
    pub fn touches_void(&self, seam: &Seam) -> bool {
        self.seam_indices.contains_key(&seam.reversed())
    }
}

impl Index<RegionKey> for Topology {
    type Output = Region;

    fn index(&self, index: RegionKey) -> &Self::Output {
        &self.regions[index]
    }
}

impl Display for Topology {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let indent = "    ";

        for (region_key, region) in self.regions.iter().enumerate() {
            writeln!(f, "Component {region_key}")?;
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
        cycle.last().unwrap().stop_vertex()
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

pub fn split_cycle_into_seams2<T: Eq>(cycle: &Vec<Side>, f: impl Fn(Side) -> T) -> Vec<Seam> {
    let discontinuities: Vec<_> = cycle
        .iter()
        .circular_tuple_windows()
        .filter(|(&lhs, &rhs)| f(lhs) != f(rhs))
        .collect();

    if discontinuities.is_empty() {
        // TODO: Reverse cycle should give same seam
        vec![Seam::new(*cycle.first().unwrap(), *cycle.last().unwrap())]
    } else {
        discontinuities
            .iter()
            .circular_tuple_windows()
            .map(|((_, &lhs), (&rhs, _))| Seam::new(lhs, rhs))
            .collect()
    }
}

/// Split a side cycle into segments based on a function f: Side -> T
/// Each segment has constant f
pub fn split_cycle_into_seams<T: Eq>(cycle: &Vec<Side>, f: impl Fn(Side) -> T) -> Vec<Seam> {
    let mut iter = cycle.iter();
    let Some(&first_side) = iter.next() else {
        return Vec::new();
    };

    // Group cycle sides into seams, (groups of constant f)
    let mut seams: Vec<Seam> = Vec::new();
    let mut seam = Seam::new(first_side, first_side);
    for &side in iter {
        if f(seam.start) == f(side) {
            seam.stop = side;
        } else {
            // Start a new seam at the current side
            seams.push(seam);
            seam = Seam::new(side, side);
        }
    }

    // Handle unfinished last seam, if f(seam) == f(first_seam), first_seam is merged with seam,
    // otherwise it is appended to seams
    match seams.first_mut() {
        Some(first_seam) if f(first_seam.start) == f(seam.stop) => first_seam.start = seam.start,
        _ => seams.push(seam),
    }

    seams
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
    fn image_2a() {
        let rgb_edges = load_rgb_seam_graph("2a.png");
        let expected_rgb_edges = rgb_edges_from([(Rgba8::RED, Rgba8::BLUE)]);
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
