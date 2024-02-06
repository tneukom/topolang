use crate::{
    bitmap::Bitmap,
    connected_components::{color_components, left_of, right_of, ColorComponent},
    math::{
        pixel::{Corner, Pixel, Side},
        point::Point,
        rect::{Rect, RectBounds},
        rgba8::Rgba8,
    },
    pixmap::Pixmap,
    utils::{UndirectedEdge, UndirectedGraph},
};
use itertools::Itertools;
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt,
    fmt::{Display, Formatter},
    ops::{Index, IndexMut},
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Seam {
    pub start: Side,
    pub stop: Side,
    pub len: usize,
}

impl Seam {
    pub fn new_atom(start: Side, stop: Side) -> Self {
        Seam {
            start,
            stop,
            len: 1,
        }
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

    pub fn start_corner(&self) -> Corner {
        self.start.start_corner()
    }

    pub fn stop_corner(&self) -> Corner {
        self.stop.stop_corner()
    }

    pub fn is_loop(&self) -> bool {
        self.start_corner() == self.stop_corner()
    }

    pub fn translated(self, offset: Point<i64>) -> Self {
        Self {
            start: self.start + offset,
            stop: self.stop + offset,
            len: self.len,
        }
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
    pub right: Option<Rgba8>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct RegionKey {
    key: usize,
}

impl Display for RegionKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "RegionKey({})", self.key)
    }
}

impl RegionKey {
    pub fn unused() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(1);
        Self {
            key: COUNTER.fetch_add(1, Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Border {
    /// First side is minimum side of cycle, contains at least one item
    pub cycle: Vec<Side>,

    /// Start side of first seam is first side in cycle, contains at least one item
    pub seams: Vec<Seam>,

    pub is_outer: bool,
}

impl Border {
    /// All pixels that are left of `self`
    pub fn left_pixels(&self) -> BTreeSet<Pixel> {
        let sides: BTreeSet<_> = self.cycle.iter().copied().collect();
        left_of(&sides).interior
    }

    /// All pixels that are right of `self`
    pub fn right_pixels(&self) -> BTreeSet<Pixel> {
        let sides: BTreeSet<_> = self.cycle.iter().copied().collect();
        right_of(&sides).interior
    }

    pub fn translate(&mut self, offset: Point<i64>) {
        for side in &mut self.cycle {
            *side = *side + offset;
        }

        for seam in &mut self.seams {
            *seam = seam.translated(offset);
        }
    }

    pub fn translated(mut self, offset: Point<i64>) -> Self {
        self.translate(offset);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegionFlags {
    /// Regions cannot be matched or overwritten.
    pub isolated: bool,
}

/// The boundary is counterclockwise. Is mutable so color can be changed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Region {
    pub color: Rgba8,
    /// First item is outer Border
    pub boundary: Vec<Border>,

    /// Contains at least one pixel
    pub interior: BTreeSet<Pixel>,

    // /// If the region is not closed left_of(boundary) is not defined otherwise
    // /// left_of(boundary) = interior
    // pub closed: bool,
    pub bounds: Rect<i64>,
}

impl Region {
    pub fn iter_seams(&self) -> impl Iterator<Item = &Seam> {
        self.boundary.iter().flat_map(|border| border.seams.iter())
    }

    /// Any `pixel` in `pixels` either
    pub fn touches(&self, pixmap: &Pixmap) -> bool {
        pixmap.keys().any(|&pixel| {
            pixel
                .touching()
                .iter()
                .any(|touched| self.interior.contains(touched))
        })
    }

    pub fn translate(&mut self, offset: Point<i64>) {
        for border in &mut self.boundary {
            border.translate(offset);
        }

        self.interior = self.interior.iter().map(|&pixel| pixel + offset).collect();
        self.bounds = self.bounds + offset;
    }

    pub fn translated(mut self, offset: Point<i64>) -> Self {
        self.translate(offset);
        self
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

impl Display for SeamIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SeamIndex({}, {}, {})",
            self.region_key, self.i_border, self.i_seam
        )
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

#[derive(Debug, Clone, Copy)]
pub struct FillRegion {
    /// Region key in the pattern, the matched region is filled with `color`
    pub region_key: RegionKey,
    pub color: Rgba8,
}

#[derive(Debug, Clone)]
pub struct Topology {
    pub regions: BTreeMap<RegionKey, Region>,

    /// Maps the start side of a seam to (region key, border index, seam index)
    pub seam_indices: BTreeMap<Side, SeamIndex>,
}

impl Topology {
    pub fn new(pixmap: &Pixmap) -> Self {
        let color_components = color_components(pixmap);

        let mut regions: BTreeMap<RegionKey, Region> = BTreeMap::new();

        for ColorComponent { component, color } in color_components {
            // Each cycle in the sides is a border
            let mut boundary = Vec::new();

            for (i_border, cycle) in split_into_cycles(&component.sides).into_iter().enumerate() {
                let seams = split_cycle_into_seams(&cycle, |side| pixmap.get(&side.right_pixel()));

                let border = Border {
                    cycle,
                    seams,
                    is_outer: i_border == 0,
                };
                boundary.push(border);
            }

            // Upper bound is exclusive
            let interior_points = component.interior.iter().map(|pixel| pixel.point());
            let bounds = RectBounds::iter_bounds(interior_points).inc_high();

            let region = Region {
                boundary,
                color,
                bounds,
                interior: component.interior,
                // closed: component.closed,
            };
            regions.insert(RegionKey::unused(), region);
        }

        Self::from_regions(regions)
    }

    pub fn from_regions(regions: BTreeMap<RegionKey, Region>) -> Self {
        let mut seam_indices: BTreeMap<Side, SeamIndex> = BTreeMap::new();

        for (&region_key, region) in &regions {
            for (i_border, border) in region.boundary.iter().enumerate() {
                for (i_seam, &seam) in border.seams.iter().enumerate() {
                    let seam_index = SeamIndex::new(region_key, i_border, i_seam);
                    seam_indices.insert(seam.start, seam_index);
                }
            }
        }

        Topology {
            regions,
            seam_indices,
        }
    }

    pub fn empty() -> Self {
        Self {
            regions: BTreeMap::new(),
            seam_indices: BTreeMap::new(),
        }
    }

    pub fn from_bitmap(bitmap: &Bitmap) -> Topology {
        let pixmap = Pixmap::from_bitmap(bitmap);
        Topology::new(&pixmap)
    }

    // pub fn from_bitmap_with_void(bitmap: &Bitmap) -> Topology {
    //     let mut pixmap = pixmap_from_bitmap(&bitmap);
    //     pixmap_remove_color(&mut pixmap, Self::VOID_COLOR);
    //     Self::new(&pixmap)
    // }

    pub fn from_bitmap_path(path: impl AsRef<Path>) -> anyhow::Result<Topology> {
        let bitmap = Bitmap::from_path(path)?;
        Ok(Self::from_bitmap(&bitmap))
    }

    fn remove_seam(&mut self, seam: &Seam) -> Option<SeamIndex> {
        self.seam_indices.remove(&seam.start)
    }

    pub fn remove_region(&mut self, key: RegionKey) -> Option<Region> {
        if let Some(region) = self.regions.remove(&key) {
            for seam in region.iter_seams() {
                self.remove_seam(seam);
            }
            Some(region)
        } else {
            None
        }
    }

    /// Undefined behaviour if region overlaps existing regions
    pub fn insert_region(&mut self, region: Region) {
        let region_key = RegionKey::unused();

        for (i_border, border) in region.boundary.iter().enumerate() {
            for (i_seam, seam) in border.seams.iter().enumerate() {
                let seam_index = SeamIndex::new(region_key, i_border, i_seam);
                let existing = self.seam_indices.insert(seam.start, seam_index);
                assert!(existing.is_none());
            }
        }

        self.regions.insert(region_key, region);
    }

    /// Upper bound is exclusive, lower bound inclusive
    pub fn bounds(&self) -> Rect<i64> {
        RectBounds::iter_bounds(self.regions.values().map(|region| region.bounds))
    }

    pub fn translated(self, offset: Point<i64>) -> Self {
        let mut regions = self.regions;
        for region in regions.values_mut() {
            region.translate(offset);
        }
        Self::from_regions(regions)
    }

    pub fn iter_borders(&self) -> impl Iterator<Item = &Border> + Clone {
        self.regions
            .values()
            .flat_map(|region| region.boundary.iter())
    }

    pub fn iter_borders_with_key(&self) -> impl Iterator<Item = (BorderKey, &Border)> + Clone {
        self.regions.iter().flat_map(|(&region_key, region)| {
            region
                .boundary
                .iter()
                .enumerate()
                .map(move |(i_border, border)| (BorderKey::new(region_key, i_border), border))
        })
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

    pub fn contains_seam(&self, seam: &Seam) -> bool {
        self.seam_indices.contains_key(&seam.start)
    }

    /// Returns the border of a given seam
    /// Only fails if seam is not self.contains_seam(seam)
    pub fn seam_border(&self, seam: &Seam) -> BorderKey {
        let seam_index = &self.seam_indices[&seam.start];
        seam_index.border_index()
    }

    /// Return the region key on the left side of a given seam
    /// Only fails if seam is not self.contains_seam(seam)
    pub fn left_of(&self, seam: &Seam) -> RegionKey {
        let seam_index = &self.seam_indices[&seam.start];
        seam_index.region_key
    }

    pub fn color_left_of(&self, seam: &Seam) -> Rgba8 {
        let left_key = self.left_of(seam);
        self[left_key].color
    }

    /// Not every seam has a region on the right, it can be empty space
    pub fn right_of(&self, seam: &Seam) -> Option<RegionKey> {
        assert!(seam.is_atom());
        let seam_index = self.seam_indices.get(&seam.reversed().start)?;
        Some(seam_index.region_key)
    }

    pub fn color_right_of(&self, seam: &Seam) -> Option<Rgba8> {
        let right_key = self.right_of(seam)?;
        Some(self[right_key].color)
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
            left: self.color_left_of(seam),
            right: self.color_right_of(seam),
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

    pub fn seams_equivalent(&self, lhs: &Seam, rhs: &Seam) -> bool {
        lhs.is_loop() && rhs.is_loop() && self.seam_border(lhs) == self.seam_border(rhs)
    }

    pub fn to_pixmap(&self) -> Pixmap {
        let mut pixmap = Pixmap::new();
        for region in self.regions.values() {
            for &pixel in &region.interior {
                pixmap.set(pixel, region.color);
            }
        }

        pixmap
    }

    #[inline(never)]
    fn fill_regions_fallback(&mut self, fill_regions: &Vec<FillRegion>) {
        let mut pixmap = self.to_pixmap();

        for fill_region in fill_regions.into_iter() {
            for &pixel in &self.regions[&fill_region.region_key].interior {
                pixmap.set(pixel, fill_region.color);
            }
        }

        *self = Self::new(&pixmap)
    }

    /// Invalidates all regions keys
    #[inline(never)]
    pub fn fill_regions(&mut self, fill_regions: &Vec<FillRegion>) {
        let mut remaining = Vec::new();
        for &fill_region in fill_regions {
            // If all neighboring regions have a different color than fill_region.color we can
            // simply replace Region.color
            let region = &self[fill_region.region_key];
            if region
                .iter_seams()
                .all(|seam| self.color_right_of(seam) != Some(fill_region.color))
            {
                let region = &mut self[fill_region.region_key];
                region.color = fill_region.color;
            } else {
                remaining.push(fill_region)
            }
        }

        if !remaining.is_empty() {
            self.fill_regions_fallback(&remaining);
        }
    }

    pub fn border_containing_side(&self, side: &Side) -> Option<(BorderKey, &Border)> {
        self.iter_borders_with_key()
            .find(|(_, border)| border.cycle.contains(side))
    }

    /// Whole are is one single component
    pub fn is_connected() -> bool {
        todo!()
    }

    /// Whole area is connected and does not contain any holes.
    /// https://en.wikipedia.org/wiki/Simply_connected_space
    pub fn is_simply_connected() -> bool {
        todo!()
    }

    /// Returns the set of all regions that are right of `border`. Only returns regions that are connected to the passed
    /// border.
    /// If there is a region on the right side of the passed border inside a hole it is not returned!
    pub fn regions_right_of_border(&self, border: &Border) -> BTreeSet<RegionKey> {
        // flood fill regions over seams
        let mut right_of: BTreeSet<RegionKey> = BTreeSet::new();

        // start with the regions directly right of the passed border.
        let mut todo: Vec<_> = border
            .seams
            .iter()
            .flat_map(|seam| self.right_of(seam))
            .collect();

        while let Some(region_key) = todo.pop() {
            if right_of.insert(region_key) {
                // new region added, add all regions touching the boundary
                for seam in self[region_key].iter_seams() {
                    if border.seams.contains(&seam.reversed()) {
                        // Don't go outside the border
                        continue;
                    }

                    if let Some(right_region_key) = self.right_of(seam) {
                        todo.push(right_region_key);
                    }
                }
            }
        }

        right_of
    }

    /// Keys of resulting Regions are unchanged
    pub fn topology_right_of_border(&self, border: &Border) -> Self {
        let regions = self.regions_right_of_border(border);
        self.sub_topology(regions)
    }

    pub fn blit(&mut self, pixmap: &Pixmap) {
        // Find regions that are touched by `pixels`
        let touched_regions: Vec<_> = self
            .regions
            .iter()
            .filter(|(_, region)| region.touches(pixmap))
            .map(|(&key, _)| key)
            .collect();

        // Extract these regions into a Pixmap
        let mut extracted_pixmap = Pixmap::new();
        for region_key in touched_regions {
            let region = self.remove_region(region_key).unwrap();
            extracted_pixmap.fill_region(&region);
        }

        // Blit pixels to the Pixmap
        extracted_pixmap.blit(pixmap);

        // Convert Pixmap back to Topology
        let topology = Topology::new(&extracted_pixmap);

        // Merge topology with self
        for region in topology.regions.into_values() {
            self.insert_region(region);
        }
    }

    pub fn fallback_blit(&mut self, pixmap: &Pixmap) {
        let mut self_pixmap = self.to_pixmap();
        self_pixmap.blit(pixmap);
        *self = Topology::new(&self_pixmap);
    }

    pub fn rgb_seam_graph(&self) -> UndirectedGraph<Option<Rgba8>> {
        let mut edges = BTreeSet::new();
        for seam in self.iter_seams() {
            let seam_colors = self.seam_colors(seam);
            let edge = UndirectedEdge::new(Some(seam_colors.left), seam_colors.right);
            edges.insert(edge);
        }

        edges
    }

    pub fn sub_topology_from_iter(&self, iter: impl Iterator<Item = RegionKey>) -> Self {
        let regions = iter.map(|key| (key, self[key].clone())).collect();
        Self::from_regions(regions)
    }

    pub fn sub_topology(&self, into_iter: impl IntoIterator<Item = RegionKey>) -> Self {
        self.sub_topology_from_iter(into_iter.into_iter())
    }

    pub fn filter_regions(self, mut predicate: impl FnMut(RegionKey, &Region) -> bool) -> Self {
        let regions = self
            .regions
            .into_iter()
            .filter(|(key, region)| predicate(*key, region))
            .collect();
        Self::from_regions(regions)
    }

    /// Remove all regions of the given color
    pub fn without_color(self, color: Rgba8) -> Self {
        self.filter_regions(|_, region| region.color != color)
    }
}

/// Warning: Slow
impl PartialEq for Topology {
    fn eq(&self, other: &Self) -> bool {
        let self_regions: HashSet<_> = self.regions.values().collect();
        let other_regions: HashSet<_> = other.regions.values().collect();
        self_regions == other_regions
    }
}

impl Index<RegionKey> for Topology {
    type Output = Region;

    fn index(&self, key: RegionKey) -> &Self::Output {
        &self.regions[&key]
    }
}

impl IndexMut<RegionKey> for Topology {
    fn index_mut(&mut self, key: RegionKey) -> &mut Self::Output {
        self.regions.get_mut(&key).unwrap()
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

// pub fn split_boundary_into_seams(
//     boundary: &Vec<Side>,
//     component_map: BTreeMap<Pixel, usize>,
// ) -> Vec<Vec<Side>> {
//     // Each group of sides has a constant right side
//     let groups: HashMap<_, _> = boundary
//         .into_iter()
//         .into_group_map_by(|side| component_map.get(&side.right_pixel()));
//
//     // We still need to split each seam into connected paths
//     for (_, group) in groups {
//
//     }
// }

/// Extract the side cycle that starts (and stops) at start_corner
pub fn extract_cycle(side_graph: &mut BTreeMap<Corner, Side>, mut corner: Corner) -> Vec<Side> {
    let mut cycle = Vec::new();
    while let Some(side) = side_graph.remove(&corner) {
        cycle.push(side);
        corner = side.stop_corner();
    }
    assert_eq!(
        cycle.first().unwrap().start_corner(),
        cycle.last().unwrap().stop_corner(),
        "borders with gaps are not allowed"
    );
    cycle
}

/// Split a set of sides into cycles. The start point of every cycle is its minimum
/// (lexicographic order x, y). `sides` must be an iterable of Side.
/// `[cycle.first() | cycle in cycles]` is ordered from small to large. This means the
/// first cycle is the outer cycle.
pub fn split_into_cycles<'a>(sides: impl IntoIterator<Item = &'a Side>) -> Vec<Vec<Side>> {
    // Maps corner to side that starts at the corner
    let mut side_graph: BTreeMap<Corner, Side> = BTreeMap::new();
    for &side in sides {
        let previous = side_graph.insert(side.start_corner(), side);
        // Should never happen
        assert_eq!(previous, None);
    }

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
    assert!(!cycle.is_empty());

    let discontinuities: Vec<_> = cycle
        .iter()
        .circular_tuple_windows()
        .filter(|(&lhs, &rhs)| f(lhs) != f(rhs))
        .collect();

    if discontinuities.is_empty() {
        // TODO: Reverse cycle should give same seam
        vec![Seam::new_atom(
            *cycle.first().unwrap(),
            *cycle.last().unwrap(),
        )]
    } else {
        discontinuities
            .iter()
            .circular_tuple_windows()
            .map(|((_, &lhs), (&rhs, _))| Seam::new_atom(lhs, rhs))
            .collect()
    }
}

#[cfg(test)]
pub mod test {
    use crate::{
        math::rgba8::Rgba8,
        pixmap::Pixmap,
        topology::{Region, Topology},
        utils::{UndirectedEdge, UndirectedGraph},
    };
    use itertools::Itertools;

    fn load_topology(filename: &str) -> Topology {
        let path = format!("test_resources/topology/{filename}");
        Topology::from_bitmap_path(path).unwrap()
    }

    fn load_rgb_seam_graph(filename: &str) -> UndirectedGraph<Option<Rgba8>> {
        let topo = load_topology(filename);
        topo.rgb_seam_graph()
    }

    fn rgb_edges_from<const N: usize>(
        rgb_edges: [(Rgba8, Rgba8); N],
    ) -> UndirectedGraph<Option<Rgba8>> {
        let colorf = |rgba| {
            if rgba == Rgba8::VOID {
                None
            } else {
                Some(rgba)
            }
        };

        rgb_edges
            .into_iter()
            .map(|(a, b)| UndirectedEdge::new(colorf(a), colorf(b)))
            .collect()
    }

    #[test]
    fn image_1a() {
        let rgb_edges = load_rgb_seam_graph("1a.png");
        let expected_rgb_edges = rgb_edges_from([
            (Rgba8::TRANSPARENT, Rgba8::VOID),
            (Rgba8::RED, Rgba8::TRANSPARENT),
        ]);
        assert_eq!(rgb_edges, expected_rgb_edges);
    }

    #[test]
    fn image_2a() {
        let rgb_edges = load_rgb_seam_graph("2a.png");
        let expected_rgb_edges = rgb_edges_from([
            (Rgba8::TRANSPARENT, Rgba8::VOID),
            (Rgba8::RED, Rgba8::BLUE),
            (Rgba8::RED, Rgba8::TRANSPARENT),
            (Rgba8::BLUE, Rgba8::TRANSPARENT),
        ]);
        assert_eq!(rgb_edges, expected_rgb_edges);
    }

    #[test]
    fn image_2b() {
        let rgb_edges = load_rgb_seam_graph("2b.png");
        let expected_rgb_edges = rgb_edges_from([
            (Rgba8::TRANSPARENT, Rgba8::VOID),
            (Rgba8::RED, Rgba8::BLUE),
            (Rgba8::RED, Rgba8::TRANSPARENT),
        ]);
        assert_eq!(rgb_edges, expected_rgb_edges);
    }

    #[test]
    fn image_2c() {
        let topology = load_topology("2c.png");
        assert_eq!(topology.regions.len(), 3);
    }

    #[test]
    fn image_3a() {
        let rgb_edges = load_rgb_seam_graph("3a.png");
        let expected_rgb_edges = rgb_edges_from([
            (Rgba8::TRANSPARENT, Rgba8::VOID),
            (Rgba8::RED, Rgba8::GREEN),
            (Rgba8::RED, Rgba8::BLUE),
            (Rgba8::RED, Rgba8::TRANSPARENT),
        ]);
        assert_eq!(rgb_edges, expected_rgb_edges);
    }

    #[test]
    fn image_3b() {
        let rgb_edges = load_rgb_seam_graph("3b.png");
        let expected_rgb_edges = rgb_edges_from([
            (Rgba8::RED, Rgba8::VOID),
            (Rgba8::RED, Rgba8::GREEN),
            (Rgba8::GREEN, Rgba8::BLUE),
        ]);
        assert_eq!(rgb_edges, expected_rgb_edges);
    }

    #[test]
    fn image_3c() {
        let rgb_edges = load_rgb_seam_graph("3c.png");
        let expected_rgb_edges = rgb_edges_from([
            (Rgba8::TRANSPARENT, Rgba8::VOID),
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
            (Rgba8::TRANSPARENT, Rgba8::VOID),
            (Rgba8::RED, Rgba8::BLUE),
            (Rgba8::BLUE, Rgba8::GREEN),
            (Rgba8::GREEN, Rgba8::CYAN),
            (Rgba8::BLUE, Rgba8::CYAN),
            (Rgba8::RED, Rgba8::CYAN),
            (Rgba8::RED, Rgba8::TRANSPARENT),
        ]);
        assert_eq!(rgb_edges, expected_rgb_edges);
    }

    fn print_region(region: &Region, indent: &str) {
        println!("{indent}color: {}", region.color);
        println!("{indent}color: {:?}", region.bounds);
    }

    fn assert_blit(name: &str) {
        let folder = format!("test_resources/topology/blit/{name}");

        let mut base = Topology::from_bitmap_path(format!("{folder}/base.png")).unwrap();

        let blit = Pixmap::from_bitmap_path(format!("{folder}/blit.png"))
            .unwrap()
            .without_color(Rgba8::TRANSPARENT);

        let mut expected_result = base.clone();
        expected_result.blit(&blit);

        base.blit(&blit);

        assert_eq!(base, expected_result);
    }

    #[test]
    fn blit_a() {
        assert_blit("a");
    }

    #[test]
    fn blit_b() {
        assert_blit("b");
    }

    #[test]
    fn blit_c() {
        assert_blit("c");
    }

    #[test]
    fn blit_d() {
        assert_blit("d");
    }

    #[test]
    fn blit_e() {
        assert_blit("e");
    }

    fn assert_right_of(name: &str) {
        let folder = "test_resources/topology/right_of";
        let topology = Topology::from_bitmap_path(format!("{folder}/{name}.png")).unwrap();

        let (_, red_region) = topology
            .regions
            .iter()
            .filter(|(_, region)| region.color == Rgba8::RED)
            .exactly_one()
            .unwrap();

        // Outer and a single interior border
        assert_eq!(red_region.boundary.len(), 2);

        // let border_key = BorderKey::new(red_region_key, 1);
        let red_interior_border = &red_region.boundary[1];
        let regions_right_of = topology.regions_right_of_border(red_interior_border);
        let right_of_topology = topology.sub_topology(regions_right_of);

        // right_of_topology
        //     .to_pixmap()
        //     .to_bitmap_with_size(Point(128, 128))
        //     .save("result.png")
        //     .unwrap();

        // Load expected result from bitmap and strip transparent pixels
        let expected_pixmap = Pixmap::from_bitmap_path(format!("{folder}/{name}_expected.png"))
            .unwrap()
            .without_color(Rgba8::TRANSPARENT);
        let expected = Topology::new(&expected_pixmap);

        assert_eq!(right_of_topology, expected);
    }

    #[test]
    fn right_of_a() {
        assert_right_of("a");
    }

    #[test]
    fn right_of_b() {
        assert_right_of("b");
    }
}
