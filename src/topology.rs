use crate::{
    cycle_segments::{CycleSegment, CycleSegments},
    material::Material,
    math::{
        pixel::{Corner, Pixel, Side},
        point::Point,
        rect::Rect,
    },
    pixmap::{MaterialMap, Pixmap},
    regions::{pixmap_regions, region_boundaries, split_boundary_into_cycles},
    utils::{UndirectedEdge, UndirectedGraph},
};
use itertools::Itertools;
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt,
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    ops::{Index, IndexMut},
};
use std::path::Path;
use crate::field::RgbaField;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeamCorner {
    Start,
    Stop,
}

impl SeamCorner {
    pub const ALL: [SeamCorner; 2] = [Self::Start, Self::Stop];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Seam {
    pub start: Side,
    pub stop: Side,

    /// Number of atomic Seams that make up this Seam.
    pub atoms: usize,
}

impl Seam {
    pub fn new_atom(start: Side, stop: Side) -> Self {
        Seam {
            start,
            stop,
            atoms: 1,
        }
    }

    pub fn new_with_len(start: Side, stop: Side, atoms: usize) -> Self {
        assert!(atoms > 0);
        Seam { start, stop, atoms }
    }

    pub fn is_atom(&self) -> bool {
        self.atoms == 1
    }

    /// The reverse of a non-atomic Seam is in general not a Seam.
    pub fn atom_reversed(&self) -> Seam {
        assert!(self.is_atom());
        Self::new_atom(self.stop.reversed(), self.start.reversed())
    }

    pub fn start_corner(&self) -> Corner {
        self.start.start_corner()
    }

    pub fn stop_corner(&self) -> Corner {
        self.stop.stop_corner()
    }

    pub fn corner(&self, name: SeamCorner) -> Corner {
        match name {
            SeamCorner::Start => self.start_corner(),
            SeamCorner::Stop => self.stop_corner(),
        }
    }

    pub fn is_loop(&self) -> bool {
        self.start_corner() == self.stop_corner()
    }

    pub fn translated(self, offset: Point<i64>) -> Self {
        Self {
            start: self.start + offset,
            stop: self.stop + offset,
            atoms: self.atoms,
        }
    }

    // self + offset == other
    pub fn translated_eq(&self, offset: Point<i64>, other: &Seam) -> bool {
        &self.translated(offset) == other
    }
}

impl Display for Seam {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Seam({} -> {}, atoms: {})", self.start, self.stop, self.atoms)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeamMaterials {
    pub left: Material,
    pub right: Option<Material>,
}

// #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
// pub struct RegionKey {
//     key: usize,
// }
//
// impl Display for RegionKey {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         write!(f, "RegionKey({})", self.key)
//     }
// }
//
// impl RegionKey {
//     pub fn unused() -> Self {
//         static COUNTER: AtomicUsize = AtomicUsize::new(1);
//         Self {
//             key: COUNTER.fetch_add(1, Ordering::Relaxed),
//         }
//     }
// }

// TODO: RegionKey should be Pixel
pub type RegionKey = usize;

pub type StrongRegionKey = Pixel;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Border {
    /// First side is minimum side of cycle, contains at least one item
    pub cycle: Vec<(Side, Option<RegionKey>)>,

    // /// Start side of first seam is first side in cycle, contains at least one item
    // pub seams: Vec<Seam>,
    pub cycle_segments: CycleSegments,

    pub is_outer: bool,
}

impl Border {
    pub fn sides(&self) -> impl DoubleEndedIterator<Item = Side> + Clone + '_ {
        self.cycle.iter().map(|(side, _)| *side)
    }

    /// self + offset == other
    pub fn translated_eq(&self, offset: Point<i64>, other: &Self) -> bool {
        if self.cycle.len() != other.cycle.len() {
            return false;
        }

        self.sides()
            .zip(other.sides())
            .all(|(self_side, other_side)| self_side + offset == other_side)
    }

    pub fn seams_len(&self) -> usize {
        self.cycle_segments.len()
    }

    fn seam_from_segment(&self, segment: CycleSegment) -> Seam {
        Seam {
            start: self.cycle[segment.first()].0,
            stop: self.cycle[segment.last()].0,
            atoms: 1,
        }
    }

    pub fn atomic_seam(&self, i: usize) -> Seam {
        self.seam_from_segment(self.cycle_segments.segment(i))
    }

    pub fn seam(&self, start: usize, atoms: usize) -> Seam {
        let start_seam = self.atomic_seam(start);
        let stop_seam = self.atomic_seam((start + atoms - 1) % self.cycle_segments.len());
        Seam::new_with_len(start_seam.start, stop_seam.stop, atoms)
    }

    /// Seam that starts and stop at `corner`
    pub fn loop_seam_with_corner(&self, corner: Corner) -> Option<Seam> {
        self.loop_seams().find(|seam| seam.start_corner() == corner)
    }

    pub fn loop_seam(&self) -> Seam {
        self.seam(0, self.cycle_segments.len())
    }

    /// All possible loop seams, they are all equivalent but start at different corners.
    pub fn loop_seams(&self) -> impl Iterator<Item = Seam> + '_ {
        let len = self.cycle_segments.len();
        (0..len).map(move |start| self.seam(start, len))
    }

    pub fn corner(&self, i: usize) -> Corner {
        self.atomic_seam(i).start_corner()
    }

    pub fn corners(&self) -> impl ExactSizeIterator<Item = Corner> + Clone + '_ {
        self.atomic_seams().map(|seam| seam.start_corner())
    }

    /// All atomic seams, a loop if the border only has one Seam
    pub fn atomic_seams(&self) -> impl ExactSizeIterator<Item = Seam> + Clone + '_ {
        self.cycle_segments
            .iter()
            .map(|segment| self.seam_from_segment(segment))
    }

    /// All seams (atomic and non-atomic) of this border that are not the full loop
    pub fn non_loop_seams(&self) -> impl Iterator<Item = Seam> + '_ {
        let len = self.cycle_segments.len();
        (0..len)
            .cartesian_product(1..len)
            .map(move |(start, atoms)| self.seam(start, atoms))
    }

    pub fn seam_between_corners(&self, start_corner: Corner, stop_corner: Corner) -> Option<Seam> {
        let (i_start, start_seam) = self
            .atomic_seams()
            .enumerate()
            .find(|(_, seam)| seam.start_corner() == start_corner)?;

        let (i_stop, stop_seam) = self
            .atomic_seams()
            .enumerate()
            .find(|(_, seam)| seam.stop_corner() == stop_corner)?;

        // start and stop seam are part of the resulting seam so it's + 1
        let atoms = (i_stop + self.seams_len() - i_start) % self.seams_len() + 1;

        Some(Seam::new_with_len(
            start_seam.start,
            stop_seam.stop,
            atoms,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Boundary {
    pub borders: Vec<Border>,
}

impl Boundary {
    pub fn new(borders: Vec<Border>) -> Self {
        assert!(!borders.is_empty());
        assert!(borders[0].is_outer);
        Self { borders }
    }

    pub fn top_left_interior_pixel(&self) -> Pixel {
        let first_border = &self.borders[0];
        assert!(first_border.is_outer);
        first_border.cycle[0].0.left_pixel
    }

    /// self + offset == other
    pub fn translated_eq(&self, offset: Point<i64>, other: &Self) -> bool {
        if self.borders.len() != other.borders.len() {
            return false;
        }

        self.borders
            .iter()
            .zip(&other.borders)
            .all(|(self_border, other_border)| self_border.translated_eq(offset, other_border))
    }
}

/// The boundary is counterclockwise. Is mutable so material can be changed.
#[derive(Debug, Clone)]
pub struct Region {
    /// First item is outer Border
    pub boundary: Boundary,

    pub material: Material,

    pub bounds: Rect<i64>,
}

impl Region {
    pub fn iter_seams(&self) -> impl Iterator<Item = Seam> + Clone + '_ {
        self.boundary
            .borders
            .iter()
            .flat_map(|border| border.atomic_seams())
    }

    pub fn top_left_interior_pixel(&self) -> Pixel {
        self.boundary.top_left_interior_pixel()
    }
}

/// Ignores `area_bounds` field
impl PartialEq for Region {
    fn eq(&self, other: &Self) -> bool {
        self.boundary == other.boundary
    }
}

/// Ignores `area_bounds` field
impl Eq for Region {}

/// Ignores `area_bounds` field
impl Hash for Region {
    fn hash<H: Hasher>(&self, state: &mut H) {
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
pub struct FillRegion {
    /// Region key in the pattern, the matched region is filled with `material`
    pub region_key: RegionKey,
    pub material: Material,
}

// pub struct Change {
//     pub time: u64,
//     pub bounds: AreaBounds,
// }

#[derive(Debug, Clone)]
pub struct Topology {
    pub material_map: MaterialMap,

    pub region_map: Pixmap<RegionKey>,

    // TODO: Make Vec<Region>
    pub regions: BTreeMap<RegionKey, Region>,

    /// Maps the start side of a seam to (region key, border index, seam index)
    pub seam_indices: BTreeMap<Side, SeamIndex>,

    bounding_rect: Rect<i64>,
}

impl Topology {
    pub fn bounding_rect(&self) -> Rect<i64> {
        self.bounding_rect
    }

    /// Bounding rect of all pixels that are not None
    pub fn not_none_bounding_rect(&self) -> Rect<i64> {
        self.material_map.not_none_bounding_rect()
    }

    #[inline(never)]
    pub fn new(material_map: MaterialMap) -> Self {
        let (region_map, region_bounding_rects) = pixmap_regions(&material_map);
        let n_regions = region_bounding_rects.len();

        let mut regions: BTreeMap<usize, Region> = BTreeMap::new();

        let region_boundaries = region_boundaries(&region_map, n_regions);

        for (region_id, (pre_boundary, region_bounding_rect)) in region_boundaries
            .into_iter()
            .zip(region_bounding_rects)
            .enumerate()
        {
            let mut borders = Vec::new();
            for (i_border, cycle) in split_boundary_into_cycles(pre_boundary)
                .into_iter()
                .enumerate()
            {
                // Each cycle is a border
                let cycle_right_side = cycle.iter().copied().map(|(_, right_side)| right_side);
                let cycle_segments = CycleSegments::from_iter(cycle_right_side);

                let border = Border {
                    cycle,
                    cycle_segments,
                    is_outer: i_border == 0,
                };
                borders.push(border);
            }

            let boundary = Boundary::new(borders);

            let region = Region {
                material: material_map
                    .get(boundary.top_left_interior_pixel())
                    .unwrap(),
                boundary: boundary,
                bounds: region_bounding_rect,
            };
            regions.insert(region_id, region);
        }

        // Build seam indices from regions
        let mut seam_indices: BTreeMap<Side, SeamIndex> = BTreeMap::new();

        for (&region_id, region) in &regions {
            for (i_border, border) in region.boundary.borders.iter().enumerate() {
                for (i_seam, seam) in border.atomic_seams().enumerate() {
                    let seam_index = SeamIndex::new(region_id, i_border, i_seam);
                    seam_indices.insert(seam.start, seam_index);
                }
            }
        }

        Self {
            bounding_rect: material_map.bounding_rect(),
            material_map,
            region_map,
            regions,
            seam_indices,
        }
    }

    pub fn translated(&self, offset: Point<i64>) -> Self {
        Self::new(self.material_map.clone().translated(offset))
    }

    pub fn iter_borders(&self) -> impl Iterator<Item = &Border> + Clone {
        self.regions
            .values()
            .flat_map(|region| region.boundary.borders.iter())
    }

    pub fn iter_borders_with_key(&self) -> impl Iterator<Item = (BorderKey, &Border)> + Clone {
        self.regions.iter().flat_map(|(&region_key, region)| {
            region
                .boundary
                .borders
                .iter()
                .enumerate()
                .map(move |(i_border, border)| (BorderKey::new(region_key, i_border), border))
        })
    }

    pub fn iter_seams(&self) -> impl Iterator<Item = Seam> + Clone + '_ {
        self.iter_borders().flat_map(|border| border.atomic_seams())
    }

    pub fn iter_seam_indices(&self) -> impl Iterator<Item = &SeamIndex> + Clone {
        self.seam_indices.values()
    }

    pub fn iter_region_keys<'a>(&'a self) -> impl Iterator<Item = &'a RegionKey> + Clone + 'a {
        self.regions.keys()
    }

    pub fn iter_region_values<'a>(&'a self) -> impl Iterator<Item = &'a Region> + Clone + 'a {
        self.regions.values()
    }

    /// Returns the index of `seam` if the exact seam exists in `self`.
    pub fn seam_index(&self, seam: Seam) -> Option<SeamIndex> {
        let &index = self.seam_indices.get(&seam.start)?;
        let region = &self.regions[&index.region_key];
        let contained_seam = region.boundary.borders[index.i_border].atomic_seam(index.i_seam);
        (contained_seam == seam).then_some(index)
    }

    /// Does the Topology contain a seam with the same start, stop and length.
    pub fn contains_seam(&self, seam: Seam) -> bool {
        self.seam_index(seam).is_some()
    }

    /// Returns the border of a given seam
    /// Only fails if seam is not self.contains_seam(seam)
    pub fn seam_border(&self, seam: Seam) -> BorderKey {
        // TODO: Use `seam_index`
        let seam_index = &self.seam_indices[&seam.start];
        seam_index.border_index()
    }

    /// Return the region key on the left side of a given seam
    /// Only fails if seam is not self.contains_seam(seam)
    pub fn left_of(&self, seam: Seam) -> RegionKey {
        // TODO: Use `seam_index`
        let seam_index = &self.seam_indices[&seam.start];
        seam_index.region_key
    }

    pub fn material_left_of(&self, seam: Seam) -> Material {
        self.material_map.get(seam.start.left_pixel).unwrap()
    }

    /// Not every seam has a region on the right, it can be empty space
    pub fn right_of(&self, seam: Seam) -> Option<RegionKey> {
        assert!(seam.is_atom());
        // TODO: Use `seam_index`
        let seam_index = self.seam_indices.get(&seam.atom_reversed().start)?;
        Some(seam_index.region_key)
    }

    pub fn material_right_of(&self, seam: Seam) -> Option<Material> {
        self.material_map.get(seam.start.right_pixel())
    }

    pub fn seam_materials(&self, seam: Seam) -> SeamMaterials {
        SeamMaterials {
            left: self.material_left_of(seam),
            right: self.material_right_of(seam),
        }
    }

    /// Seam between left and right region
    /// Component index errors cause panic
    pub fn seams_between(
        &self,
        left: RegionKey,
        right: RegionKey,
    ) -> impl Iterator<Item = Seam> + Clone + '_ {
        let left_comp = &self.regions[&left];
        left_comp
            .iter_seams()
            .filter(move |&seam| self.right_of(seam) == Some(right))
    }

    pub fn touched_regions(&self, pixels: impl IntoIterator<Item = Pixel>) -> BTreeSet<RegionKey> {
        pixels
            .into_iter()
            .flat_map(|pixel| pixel.touching())
            .flat_map(|touched| self.region_map.get(touched))
            .collect()
    }

    /// Is the right side of the seam void? The left side is always a proper region.
    pub fn touches_void(&self, seam: Seam) -> bool {
        self.right_of(seam).is_none()
    }

    pub fn seams_equivalent(&self, lhs: Seam, rhs: Seam) -> bool {
        lhs.is_loop() && rhs.is_loop() && self.seam_border(lhs) == self.seam_border(rhs)
    }

    pub fn iter_region_interior<'a>(
        &'a self,
        region_key: RegionKey,
    ) -> impl Iterator<Item = Pixel> + Clone + 'a {
        let region = &self.regions[&region_key];
        self.region_map.iter_where_value(region.bounds, region_key)
    }

    pub fn region_at(&self, pixel: Pixel) -> Option<RegionKey> {
        self.region_map.get(pixel)
    }

    pub fn border_containing_side(&self, side: &Side) -> Option<(BorderKey, &Border)> {
        self.iter_borders_with_key()
            .find(|(_, border)| border.sides().contains(side))
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
        Self::new(self.material_map.right_of_border(border))
    }

    pub fn filter_by_material(&self, mut pred: impl FnMut(Material) -> bool) -> Self {
        Self::new(self.material_map.filter(|_, material| pred(material)))
    }

    /// Blit the given region to the material_map with the given material.
    pub fn fill_region(
        &self,
        region_key: RegionKey,
        material: Material,
        material_map: &mut MaterialMap,
    ) {
        for pixel in self.iter_region_interior(region_key) {
            material_map.set(pixel, material);
        }
    }

    /// Try to set the material of `region_key` to `material`, returns true if successful.
    pub fn try_set_region_material(&mut self, region_key: RegionKey, material: Material) -> bool {
        // Try to update the topology to the changed material map
        let cat_set = self[region_key]
            .iter_seams()
            .all(|seam| self.material_right_of(seam) != Some(material));

        if !cat_set {
            return false;
        }

        self[region_key].material = material;
        true
    }

    pub fn material_seam_graph(&self) -> UndirectedGraph<Option<Material>> {
        let mut edges = BTreeSet::new();
        for seam in self.iter_seams() {
            let seam_materials = self.seam_materials(seam);
            let edge = UndirectedEdge::new(Some(seam_materials.left), seam_materials.right);
            edges.insert(edge);
        }

        edges
    }

    pub fn are_seams_overlapping(&self, lhs: Seam, rhs: Seam) -> bool {
        let lhs_index = self.seam_indices[&lhs.start];
        let rhs_index = self.seam_indices[&rhs.start];
        if lhs_index.border_index() != rhs_index.border_index() {
            println!("lhs_index.border_index() != rhs_index.border_index()");
            return false;
        }

        // Is there an i such that lhs_index.i_seam <= i <= lhs_index.i_seam + lhs.atoms
        // and rhs_index.i_seam <= i < rhs_index.i_seam + rhs.atoms
        // max(lhs_index.i_seam, rhs_index.i_seam) <= i
        // and i < min(lhs_index.i_seam + lhs.atoms, rhs_index.i_seam + rhs.atoms)
        lhs_index.i_seam.max(rhs_index.i_seam)
            < (lhs_index.i_seam + lhs.atoms).min(rhs_index.i_seam + rhs.atoms)
    }

    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Topology> {
        let rgba_field = RgbaField::load(path)?;
        let material_map = MaterialMap::from(rgba_field);
        Ok(Topology::new(material_map))
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
        &self.regions[&key.region_key].boundary.borders[key.i_border]
    }
}

impl Display for Topology {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let indent = "    ";

        for (region_key, region) in self.regions.iter() {
            writeln!(f, "Component {region_key:?}")?;
            for (i_border, border) in region.boundary.borders.iter().enumerate() {
                writeln!(f, "{indent}Border {i_border}")?;
                for seam in border.atomic_seams() {
                    writeln!(f, "{indent}{indent}{seam}")?;
                }
            }
        }

        Ok(())
    }
}

impl From<MaterialMap> for Topology {
    fn from(material_map: MaterialMap) -> Self {
        Self::new(material_map)
    }
}

#[cfg(test)]
pub mod test {
    use crate::{
        material::Material,
        math::rgba8::Rgba8,
        topology::Topology,
        utils::{UndirectedEdge, UndirectedGraph},
    };

    fn load_topology(filename: &str) -> Topology {
        let path = format!("test_resources/topology/{filename}");
        Topology::load(path).unwrap()
    }

    fn load_rgb_seam_graph(filename: &str) -> UndirectedGraph<Option<Material>> {
        let topo = load_topology(filename);
        topo.material_seam_graph()
    }

    fn rgb_edges_from<const N: usize>(
        rgb_edges: [(Rgba8, Rgba8); N],
    ) -> UndirectedGraph<Option<Material>> {
        let colorf = |rgba| {
            let material = Material::from(rgba);
            if material == Material::VOID {
                None
            } else {
                Some(material)
            }
        };

        rgb_edges
            .into_iter()
            .map(|(a, b)| UndirectedEdge::new(colorf(a), colorf(b)))
            .collect()
    }

    #[test]
    fn seam_graph_1a() {
        let edges = load_rgb_seam_graph("1a.png");

        // let topo = load_topology("1a.png");
        // For debugging!
        // for (region_key, region) in &topo.regions {
        //     println!(
        //         "region key = {:?}, color = {:?}",
        //         region_key, region.material
        //     );
        //     for border in &region.boundary {
        //         println!("  border:");
        //         for side in &border.cycle {
        //             println!("    side: {}", side);
        //             println!(
        //                 "    right side: {:?}",
        //                 topo.region_map.get(side.right_pixel())
        //             );
        //         }
        //
        //         for seam in &border.seams {
        //             println!("  seam with start={} and stop={}", seam.start, seam.stop);
        //         }
        //     }
        // }

        let expected_edges = rgb_edges_from([
            (Rgba8::TRANSPARENT, Material::RULE_BEFORE_COLOR),
            (Rgba8::RED, Rgba8::TRANSPARENT),
        ]);
        assert_eq!(edges, expected_edges);
    }

    #[test]
    fn seam_graph_2a() {
        let rgb_edges = load_rgb_seam_graph("2a.png");
        let expected_rgb_edges = rgb_edges_from([
            (Rgba8::TRANSPARENT, Material::RULE_BEFORE_COLOR),
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
            (Rgba8::TRANSPARENT, Material::RULE_BEFORE_COLOR),
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
            (Rgba8::TRANSPARENT, Material::RULE_BEFORE_COLOR),
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
            (Rgba8::RED, Material::RULE_BEFORE_COLOR),
            (Rgba8::RED, Rgba8::GREEN),
            (Rgba8::GREEN, Rgba8::BLUE),
        ]);
        assert_eq!(rgb_edges, expected_rgb_edges);
    }

    #[test]
    fn seam_graph_3c() {
        let rgb_edges = load_rgb_seam_graph("3c.png");
        let expected_rgb_edges = rgb_edges_from([
            (Rgba8::TRANSPARENT, Material::RULE_BEFORE_COLOR),
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
            (Rgba8::TRANSPARENT, Material::RULE_BEFORE_COLOR),
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
