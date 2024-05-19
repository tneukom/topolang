use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt,
    fmt::{Display, Formatter},
    hash::{Hash, Hasher},
    ops::{Index, IndexMut},
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

use itertools::Itertools;

use crate::{
    area_cover::AreaCover,
    connected_components::{
        color_components_subset, left_of_border, right_of_border, split_into_cycles,
    },
    field::RgbaField,
    material::Material,
    math::{
        pixel::{Corner, Pixel, Side},
        point::Point,
        rect::Rect,
        rgba8::Rgba8,
    },
    pixmap::{Pixmap, PixmapRgba},
    utils::{IteratorPlus, UndirectedEdge, UndirectedGraph},
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
pub struct SeamMaterials<M> {
    pub left: M,
    pub right: Option<M>,
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
    pub fn left_pixels(&self) -> Vec<Pixel> {
        left_of_border(&self.cycle)
    }

    /// All pixels that are right of `self`
    pub fn right_pixels(&self) -> Vec<Pixel> {
        right_of_border(&self.cycle)
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

/// The boundary is counterclockwise. Is mutable so material can be changed.
#[derive(Debug, Clone)]
pub struct Region<M> {
    /// First item is outer Border
    pub boundary: Vec<Border>,

    pub cover: AreaCover,

    pub material: M,
}

impl<M: Eq + Copy> Region<M> {
    pub fn iter_seams(&self) -> impl IteratorPlus<&Seam> {
        self.boundary.iter().flat_map(|border| border.seams.iter())
    }

    pub fn iter_boundary_sides<'a>(&'a self) -> impl IteratorPlus<Side> + 'a {
        self.boundary
            .iter()
            .flat_map(|border| border.cycle.iter())
            .copied()
    }

    pub fn translate(&mut self, offset: Point<i64>) {
        for border in &mut self.boundary {
            border.translate(offset);
        }
    }

    pub fn translated(mut self, offset: Point<i64>) -> Self {
        self.translate(offset);
        self
    }

    /// Pixels touching border on the left side, each pixel might appear more than once.
    pub fn interior_padding<'a>(&'a self) -> impl IteratorPlus<Pixel> + 'a {
        self.iter_boundary_sides().map(|side| side.left_pixel())
    }
}

/// Ignores `area_bounds` field
impl<M: PartialEq> PartialEq for Region<M> {
    fn eq(&self, other: &Self) -> bool {
        self.material == other.material && self.boundary == other.boundary
    }
}

/// Ignores `area_bounds` field
impl<M: Eq> Eq for Region<M> {}

/// Ignores `area_bounds` field
impl<M: Hash> Hash for Region<M> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.material.hash(state);
        self.boundary.hash(state);
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
pub struct FillRegion<M> {
    /// Region key in the pattern, the matched region is filled with `material`
    pub region_key: RegionKey,
    pub material: M,
}

// pub struct Change {
//     pub time: u64,
//     pub bounds: AreaBounds,
// }

#[derive(Debug, Clone)]
pub struct Topology<M> {
    pub regions: BTreeMap<RegionKey, Region<M>>,

    pub region_map: Pixmap<RegionKey>,

    /// Maps the start side of a seam to (region key, border index, seam index)
    pub seam_indices: BTreeMap<Side, SeamIndex>,
    // pub change_log: Vec<Change>
    bounding_rect: Rect<i64>,
}

impl<M: Eq + Copy> Topology<M> {
    pub fn bounding_rect(&self) -> Rect<i64> {
        self.bounding_rect
    }

    #[inline(never)]
    pub fn new(material_map: &Pixmap<M>) -> Self {
        Self::new_subset(material_map, material_map.keys())
    }

    #[inline(never)]
    pub fn new_subset(material_map: &Pixmap<M>, subset: impl IntoIterator<Item = Pixel>) -> Self {
        let (pre_regions, region_map) =
            color_components_subset(material_map, subset, RegionKey::unused);

        let mut regions: BTreeMap<RegionKey, Region<M>> = BTreeMap::new();

        for pre_region in pre_regions {
            // Each cycle in the sides is a border
            let mut boundary = Vec::new();

            for (i_border, cycle) in split_into_cycles(pre_region.sides).into_iter().enumerate() {
                let seams =
                    split_cycle_into_seams(&cycle, |side| material_map.get(side.right_pixel()));

                let border = Border {
                    cycle,
                    seams,
                    is_outer: i_border == 0,
                };
                boundary.push(border);
            }

            let region = Region {
                boundary,
                cover: pre_region.cover,
                material: pre_region.color,
            };
            regions.insert(pre_region.id, region);
        }

        Self::from_regions(regions, region_map)
    }

    /// Seams are not recomputed, make sure to pass valid Seams!
    #[inline(never)]
    fn from_regions(
        regions: BTreeMap<RegionKey, Region<M>>,
        region_map: Pixmap<RegionKey>,
    ) -> Self {
        let mut seam_indices: BTreeMap<Side, SeamIndex> = BTreeMap::new();

        for (&region_key, region) in &regions {
            for (i_border, border) in region.boundary.iter().enumerate() {
                for (i_seam, &seam) in border.seams.iter().enumerate() {
                    let seam_index = SeamIndex::new(region_key, i_border, i_seam);
                    seam_indices.insert(seam.start, seam_index);
                }
            }
        }

        let bounding_rect = region_map.bounding_rect();

        Topology {
            region_map,
            regions,
            seam_indices,
            bounding_rect,
        }
    }

    fn remove_seam(&mut self, seam: &Seam) -> Option<SeamIndex> {
        self.seam_indices.remove(&seam.start)
    }

    pub fn remove_region(&mut self, key: RegionKey) -> Option<Region<M>> {
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
    fn insert_region(&mut self, key: RegionKey, region: Region<M>) {
        for (i_border, border) in region.boundary.iter().enumerate() {
            for (i_seam, seam) in border.seams.iter().enumerate() {
                let seam_index = SeamIndex::new(key, i_border, i_seam);
                let existing = self.seam_indices.insert(seam.start, seam_index);
                assert!(existing.is_none());
            }
        }

        self.regions.insert(key, region);
    }

    pub fn translated(self, offset: Point<i64>) -> Self {
        let mut regions = self.regions;
        for region in regions.values_mut() {
            region.translate(offset);
        }
        let region_map = self.region_map.translated(offset);
        Self::from_regions(regions, region_map)
    }

    pub fn iter_borders(&self) -> impl IteratorPlus<&Border> {
        self.regions
            .values()
            .flat_map(|region| region.boundary.iter())
    }

    pub fn iter_borders_with_key(&self) -> impl IteratorPlus<(BorderKey, &Border)> {
        self.regions.iter().flat_map(|(&region_key, region)| {
            region
                .boundary
                .iter()
                .enumerate()
                .map(move |(i_border, border)| (BorderKey::new(region_key, i_border), border))
        })
    }

    pub fn iter_seams(&self) -> impl IteratorPlus<&Seam> {
        self.iter_borders().flat_map(|border| border.seams.iter())
    }

    pub fn iter_seam_indices(&self) -> impl IteratorPlus<&SeamIndex> {
        self.seam_indices.values()
    }

    pub fn iter_region_keys<'a>(&'a self) -> impl IteratorPlus<&RegionKey> + 'a {
        self.regions.keys()
    }

    pub fn iter_region_values<'a>(&'a self) -> impl IteratorPlus<&Region<M>> + 'a {
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

    pub fn material_left_of(&self, seam: &Seam) -> M {
        let left_key = self.left_of(seam);
        self[left_key].material
    }

    /// Not every seam has a region on the right, it can be empty space
    pub fn right_of(&self, seam: &Seam) -> Option<RegionKey> {
        assert!(seam.is_atom());
        let seam_index = self.seam_indices.get(&seam.reversed().start)?;
        Some(seam_index.region_key)
    }

    pub fn material_right_of(&self, seam: &Seam) -> Option<M> {
        let right_key = self.right_of(seam)?;
        Some(self[right_key].material)
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

    pub fn seam_materials(&self, seam: &Seam) -> SeamMaterials<M> {
        SeamMaterials {
            left: self.material_left_of(seam),
            right: self.material_right_of(seam),
        }
    }

    /// Seam between left and right region
    /// Component index errors cause panic
    pub fn seams_between(&self, left: RegionKey, right: RegionKey) -> impl IteratorPlus<&Seam> {
        let left_comp = &self.regions[&left];
        left_comp
            .iter_seams()
            .filter(move |&seam| self.right_of(seam) == Some(right))
    }

    pub fn touched_regions(&self, pixels: impl IntoIterator<Item = Pixel>) -> BTreeSet<RegionKey> {
        pixels
            .into_iter()
            .flat_map(|pixel| pixel.touching())
            .flat_map(|touched| self.region_map.get(touched).copied())
            .collect()
    }

    /// Is the right side of the seam void? The left side is always a proper region.
    pub fn touches_void(&self, seam: &Seam) -> bool {
        self.right_of(seam).is_none()
    }

    pub fn seams_equivalent(&self, lhs: &Seam, rhs: &Seam) -> bool {
        lhs.is_loop() && rhs.is_loop() && self.seam_border(lhs) == self.seam_border(rhs)
    }

    /// WARNING: Expensive
    pub fn material_map(&self) -> Pixmap<M> {
        self.region_map
            .map(|region_id| self.regions[region_id].material)
    }

    pub fn iter_region_interior<'a>(
        &'a self,
        region_key: RegionKey,
    ) -> impl IteratorPlus<Pixel> + 'a {
        let region = &self.regions[&region_key];
        self.region_map
            .iter_cover(&region.cover)
            .filter(move |(_, &iter_region_key)| iter_region_key == region_key)
            .map(|kv| kv.0)
    }

    pub fn region_at(&self, pixel: Pixel) -> Option<RegionKey> {
        self.region_map.get(pixel).copied()
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

    /// Keys of resulting Regions are unchanged
    /// Undefined behaviour if border is not part of self.
    #[inline(never)]
    pub fn topology_right_of_border(&self, border: &Border) -> Self {
        let region_map = self.region_map.right_of_border(border);
        // TODO:SPEEDUP: Should be possible to do this faster some way
        let region_keys: BTreeSet<_> = region_map.values().copied().collect();

        let mut regions = BTreeMap::new();
        for region_key in region_keys {
            regions.insert(region_key, self.regions[&region_key].clone());
        }

        Self::from_regions(regions, region_map)
    }

    #[inline(never)]
    pub fn sub_topology(&self, sub_regions: &BTreeSet<RegionKey>) -> Self {
        let region_map = self
            .region_map
            .filter(|region_key| sub_regions.contains(region_key));

        let regions = sub_regions
            .iter()
            .map(|&key| (key, self[key].clone()))
            .collect();

        Self::from_regions(regions, region_map)
    }

    /// Remove all regions of the given material
    pub fn without_material(self, material: M) -> Self {
        let sub_regions: BTreeSet<_> = self
            .regions
            .iter()
            .filter_map(|(&key, region)| {
                if region.material == material {
                    // discard this region, we need to make sure it does not border void
                    assert!(region
                        .iter_seams()
                        .all(|seam| self.material_right_of(seam).is_some()));
                    None
                } else {
                    Some(key)
                }
            })
            .collect();

        self.sub_topology(&sub_regions)
    }

    /// Blit the given region to the material_map with the given material.
    pub fn fill_region(&self, region_key: RegionKey, material: M, material_map: &mut Pixmap<M>) {
        let region = &self[region_key];
        material_map.blit_op(
            &self.region_map,
            &region.cover,
            |blit_material, &blit_region| {
                if blit_region == region_key {
                    *blit_material = Some(material);
                }
            },
        );
    }
}

impl Topology<Rgba8> {
    pub fn from_bitmap(bitmap: &RgbaField) -> Self {
        let pixmap = PixmapRgba::from_field(bitmap);
        Topology::new(&pixmap)
    }

    pub fn load_bitmap(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let bitmap = RgbaField::load(path)?;
        Ok(Self::from_bitmap(&bitmap))
    }
}

impl Topology<Material> {
    #[deprecated]
    pub fn from_bitmap(bitmap: &RgbaField) -> Self {
        let pixmap = PixmapRgba::from_field(bitmap).into_material();
        Topology::new(&pixmap)
    }

    #[deprecated]
    pub fn load_bitmap(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let bitmap = RgbaField::load(path)?;
        Ok(Self::from_bitmap(&bitmap))
    }
}

impl<M: Copy + Eq + Ord> Topology<M> {
    pub fn material_seam_graph(&self) -> UndirectedGraph<Option<M>> {
        let mut edges = BTreeSet::new();
        for seam in self.iter_seams() {
            let seam_materials = self.seam_materials(seam);
            let edge = UndirectedEdge::new(Some(seam_materials.left), seam_materials.right);
            edges.insert(edge);
        }

        edges
    }
}

/// Warning: Slow
impl<M: Eq + Hash> PartialEq for Topology<M> {
    fn eq(&self, other: &Self) -> bool {
        let self_regions: HashSet<_> = self.regions.values().collect();
        let other_regions: HashSet<_> = other.regions.values().collect();
        self_regions == other_regions
    }
}

impl<M> Index<RegionKey> for Topology<M> {
    type Output = Region<M>;

    fn index(&self, key: RegionKey) -> &Self::Output {
        &self.regions[&key]
    }
}

impl<M> IndexMut<RegionKey> for Topology<M> {
    fn index_mut(&mut self, key: RegionKey) -> &mut Self::Output {
        self.regions.get_mut(&key).unwrap()
    }
}

impl<M> Index<BorderKey> for Topology<M> {
    type Output = Border;

    fn index(&self, key: BorderKey) -> &Self::Output {
        &self.regions[&key.region_key].boundary[key.i_border]
    }
}

impl<M: Display> Display for Topology<M> {
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
        topology::Topology,
        utils::{UndirectedEdge, UndirectedGraph},
    };

    fn load_topology(filename: &str) -> Topology<Rgba8> {
        let path = format!("test_resources/topology/{filename}");
        Topology::<Rgba8>::load_bitmap(path).unwrap()
    }

    fn load_rgb_seam_graph(filename: &str) -> UndirectedGraph<Option<Rgba8>> {
        let topo = load_topology(filename);
        topo.material_seam_graph()
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
    fn seam_graph_1a() {
        let rgb_edges = load_rgb_seam_graph("1a.png");
        let expected_rgb_edges = rgb_edges_from([
            (Rgba8::TRANSPARENT, Rgba8::VOID),
            (Rgba8::RED, Rgba8::TRANSPARENT),
        ]);
        assert_eq!(rgb_edges, expected_rgb_edges);
    }

    #[test]
    fn seam_graph_2a() {
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
    fn seam_graph_2b() {
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
    fn seam_graph_3a() {
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
    fn seam_graph_3b() {
        let rgb_edges = load_rgb_seam_graph("3b.png");
        let expected_rgb_edges = rgb_edges_from([
            (Rgba8::RED, Rgba8::VOID),
            (Rgba8::RED, Rgba8::GREEN),
            (Rgba8::GREEN, Rgba8::BLUE),
        ]);
        assert_eq!(rgb_edges, expected_rgb_edges);
    }

    #[test]
    fn seam_graph_3c() {
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
    fn seam_graph_4a() {
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
}
