use crate::{
    area_cover::AreaCover,
    cycle_segments::{CycleSegment, CycleSegments},
    material::Material,
    math::{
        pixel::{Corner, Pixel, Side},
        point::Point,
        rect::Rect,
    },
    pixmap::{MaterialMap, Pixmap},
    regions::{
        left_of_boundary, pixmap_regions, region_boundaries, right_of_boundary,
        split_boundary_into_cycles,
    },
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

    pub fn new_with_len(start: Side, stop: Side, len: usize) -> Self {
        Seam {
            start,
            stop,
            atoms: len,
        }
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
        write!(f, "Seam({} -> {})", self.start, self.stop)
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

    /// All pixels that are left of `self`
    pub fn left_pixels(&self) -> Vec<Pixel> {
        left_of_boundary(self.sides())
    }

    /// All pixels that are right of `self`
    pub fn right_pixels(&self) -> Vec<Pixel> {
        right_of_boundary(self.sides())
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

    pub fn seam(&self, i: usize) -> Seam {
        self.seam_from_segment(self.cycle_segments.segment(i))
    }

    pub fn iter_seams(&self) -> impl Iterator<Item = Seam> + Clone + '_ {
        self.cycle_segments
            .iter()
            .map(|segment| self.seam_from_segment(segment))
    }

    pub fn iter_atomic_seam_sides(
        &self,
        i_seam: usize,
    ) -> impl DoubleEndedIterator<Item = Side> + Clone + '_ {
        let segment = self.cycle_segments.segment(i_seam);
        segment.iter().map(|i_side| self.cycle[i_side].0)
    }

    /// Iterate over the side of a seam (atomic or not)
    pub fn iter_seam_sides(
        &self,
        start_seam: usize,
        count: usize,
    ) -> impl DoubleEndedIterator<Item = Side> + Clone + '_ {
        (0..count).flat_map(move |offset| {
            let i_seam = (start_seam + offset) % self.cycle_segments.len();
            self.iter_atomic_seam_sides(i_seam)
        })
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

    pub fn arbitrary_interior_pixel(&self) -> Pixel {
        self.top_left_interior_pixel()
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

    pub fn iter_seams(&self) -> impl Iterator<Item = Seam> + Clone + '_ {
        self.borders.iter().flat_map(|border| border.iter_seams())
    }

    pub fn iter_sides(&self) -> impl Iterator<Item = Side> + Clone + '_ {
        self.borders.iter().flat_map(|border| border.sides())
    }
}

/// The boundary is counterclockwise. Is mutable so material can be changed.
#[derive(Debug, Clone)]
pub struct Region {
    /// First item is outer Border
    pub boundary: Boundary,

    pub cover: AreaCover,

    pub material: Material,

    pub bounds: Rect<i64>,
}

impl Region {
    #[deprecated = "Use Boundary::iter_seams"]
    pub fn iter_seams(&self) -> impl Iterator<Item = Seam> + Clone + '_ {
        self.boundary.iter_seams()
    }

    #[deprecated = "Use Boundary::iter_sides"]
    pub fn iter_boundary_sides(&self) -> impl Iterator<Item = Side> + Clone + '_ {
        self.boundary.iter_sides()
    }

    /// Pixels touching border on the left side, each pixel might appear more than once.
    pub fn interior_padding<'a>(&'a self) -> impl Iterator<Item = Pixel> + Clone + 'a {
        self.iter_boundary_sides().map(|side| side.left_pixel())
    }

    pub fn arbitrary_interior_pixel(&self) -> Pixel {
        self.boundary.arbitrary_interior_pixel()
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

    #[inline(never)]
    pub fn new(material_map: MaterialMap) -> Self {
        let (region_map, area_covers) = pixmap_regions(&material_map);
        let n_regions = area_covers.len();

        let mut regions: BTreeMap<usize, Region> = BTreeMap::new();

        let region_boundaries = region_boundaries(&region_map, n_regions);

        for (region_id, (pre_boundary, area_cover)) in
            region_boundaries.into_iter().zip(area_covers).enumerate()
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

            let bounds = Rect::index_bounds(region_map.iter_where_value(&area_cover, region_id));

            let region = Region {
                material: material_map[boundary.arbitrary_interior_pixel()],
                boundary: boundary,
                cover: area_cover,
                bounds,
            };
            regions.insert(region_id, region);
        }

        // Build seam indices from regions
        let mut seam_indices: BTreeMap<Side, SeamIndex> = BTreeMap::new();

        for (&region_id, region) in &regions {
            for (i_border, border) in region.boundary.borders.iter().enumerate() {
                for (i_seam, seam) in border.iter_seams().enumerate() {
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
        Self::new(self.material_map.translated(offset))
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
        self.iter_borders().flat_map(|border| border.iter_seams())
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
        let contained_seam = region.boundary.borders[index.i_border].seam(index.i_seam);
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
        self.material_map[seam.start.left_pixel]
    }

    /// Not every seam has a region on the right, it can be empty space
    pub fn right_of(&self, seam: Seam) -> Option<RegionKey> {
        assert!(seam.is_atom());
        // TODO: Use `seam_index`
        let seam_index = self.seam_indices.get(&seam.atom_reversed().start)?;
        Some(seam_index.region_key)
    }

    pub fn material_right_of(&self, seam: Seam) -> Option<Material> {
        self.material_map.get(seam.start.right_pixel()).cloned()
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
            .flat_map(|touched| self.region_map.get(touched).copied())
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
        self.region_map.iter_where_value(&region.cover, region_key)
    }

    pub fn region_at(&self, pixel: Pixel) -> Option<RegionKey> {
        self.region_map.get(pixel).copied()
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

    /// Remove all regions of the given material
    pub fn without_material(self, material: Material) -> Self {
        Self::new(self.material_map.without(&material))
    }

    /// Blit the given region to the material_map with the given material.
    pub fn fill_region(
        &self,
        region_key: RegionKey,
        material: Material,
        material_map: &mut MaterialMap,
    ) {
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

    pub fn material_seam_graph(&self) -> UndirectedGraph<Option<Material>> {
        let mut edges = BTreeSet::new();
        for seam in self.iter_seams() {
            let seam_materials = self.seam_materials(seam);
            let edge = UndirectedEdge::new(Some(seam_materials.left), seam_materials.right);
            edges.insert(edge);
        }

        edges
    }

    /// See Border::iter_seam_sides
    pub fn iter_seam_sides(
        &self,
        seam: Seam,
    ) -> Option<impl DoubleEndedIterator<Item = Side> + Clone + '_> {
        let index = self.seam_index(seam)?;
        let region = &self.regions[&index.region_key];
        let border = &region.boundary.borders[index.i_border];
        Some(border.iter_seam_sides(index.i_seam, seam.atoms))
    }

    // Split a Seam into atoms
    pub fn seam_atoms(&self, seam: Seam) -> Option<impl ExactSizeIterator<Item = Seam> + '_> {
        let &first_index = self.seam_indices.get(&seam.start)?;
        let region = &self.regions[&first_index.region_key];
        let border = &region.boundary.borders[first_index.i_border];

        let iter = (0..seam.atoms).map(move |offset| {
            let i_seam = (first_index.i_seam + offset) % border.seams_len();
            border.seam(i_seam)
        });
        Some(iter)
    }

    /// Unlike Seam::atom_reversed, this also works not non-atomic Seams, however it returns
    /// a list of Seams
    pub fn reverse_seam(&self, seam: Seam) -> Option<impl ExactSizeIterator<Item = Seam> + '_> {
        let iter = self
            .seam_atoms(seam)?
            .map(|seam_atom| seam_atom.atom_reversed());
        Some(iter)
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
                for seam in border.iter_seams() {
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
        field::RgbaField,
        material::Material,
        math::rgba8::Rgba8,
        topology::Topology,
        utils::{UndirectedEdge, UndirectedGraph},
    };

    fn load_topology(filename: &str) -> Topology {
        let path = format!("test_resources/topology/{filename}");
        let material_map = RgbaField::load(path).unwrap().into();
        Topology::new(material_map)
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
            (Rgba8::TRANSPARENT, Material::VOID_COLOR),
            (Rgba8::RED, Rgba8::TRANSPARENT),
        ]);
        assert_eq!(edges, expected_edges);
    }

    #[test]
    fn seam_graph_2a() {
        let rgb_edges = load_rgb_seam_graph("2a.png");
        let expected_rgb_edges = rgb_edges_from([
            (Rgba8::TRANSPARENT, Material::VOID_COLOR),
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
            (Rgba8::TRANSPARENT, Material::VOID_COLOR),
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
            (Rgba8::TRANSPARENT, Material::VOID_COLOR),
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
            (Rgba8::RED, Material::VOID_COLOR),
            (Rgba8::RED, Rgba8::GREEN),
            (Rgba8::GREEN, Rgba8::BLUE),
        ]);
        assert_eq!(rgb_edges, expected_rgb_edges);
    }

    #[test]
    fn seam_graph_3c() {
        let rgb_edges = load_rgb_seam_graph("3c.png");
        let expected_rgb_edges = rgb_edges_from([
            (Rgba8::TRANSPARENT, Material::VOID_COLOR),
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
            (Rgba8::TRANSPARENT, Material::VOID_COLOR),
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
