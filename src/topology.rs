use itertools::Itertools;
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt,
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    ops::{Index, IndexMut},
};

use crate::{
    area_cover::AreaCover,
    connected_components::{left_of_border, right_of_border},
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

    /// Start side of first seam is first side in cycle, contains at least one item
    pub seams: Vec<Seam>,

    pub is_outer: bool,
}

impl Border {
    pub fn sides(&self) -> impl DoubleEndedIterator<Item = Side> + Clone + '_ {
        self.cycle.iter().map(|(side, _)| *side)
    }

    /// All pixels that are left of `self`
    pub fn left_pixels(&self) -> Vec<Pixel> {
        left_of_border(self.sides())
    }

    /// All pixels that are right of `self`
    pub fn right_pixels(&self) -> Vec<Pixel> {
        right_of_border(self.sides())
    }

    /// self + offset == other
    pub fn eq_with_translation(&self, other: &Self, offset: Point<i64>) -> bool {
        if self.cycle.len() != other.cycle.len() {
            return true;
        }

        self.sides()
            .zip(other.sides())
            .all(|(self_side, other_side)| self_side + offset == other_side)
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

    /// Returns true if the left side is equal to the right side plus an offset, ignores material.
    pub fn is_translation_of(&self, other: &Self) -> bool {
        if self.borders.len() != other.borders.len() {
            return false;
        }

        // self + offset == other
        let offset = self.top_left_interior_pixel() - other.top_left_interior_pixel();
        self.borders
            .iter()
            .zip(&other.borders)
            .all(|(self_border, other_border)| {
                self_border.eq_with_translation(other_border, offset)
            })
    }
}

/// The boundary is counterclockwise. Is mutable so material can be changed.
#[derive(Debug, Clone)]
pub struct Region {
    /// First item is outer Border
    pub boundary: Boundary,

    pub cover: AreaCover,

    pub material: Material,
}

impl Region {
    pub fn iter_seams(&self) -> impl Iterator<Item = &Seam> + Clone {
        self.boundary
            .borders
            .iter()
            .flat_map(|border| border.seams.iter())
    }

    pub fn iter_boundary_sides(&self) -> impl Iterator<Item = Side> + Clone + '_ {
        self.boundary
            .borders
            .iter()
            .flat_map(|border| border.sides())
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
                let seams = split_cycle_into_seams(&cycle);

                let border = Border {
                    cycle,
                    seams,
                    is_outer: i_border == 0,
                };
                borders.push(border);
            }

            let boundary = Boundary::new(borders);

            let region = Region {
                material: material_map[boundary.arbitrary_interior_pixel()],
                boundary: boundary,
                cover: area_cover,
            };
            regions.insert(region_id, region);
        }

        // Build seam indices from regions
        let mut seam_indices: BTreeMap<Side, SeamIndex> = BTreeMap::new();

        for (&region_id, region) in &regions {
            for (i_border, border) in region.boundary.borders.iter().enumerate() {
                for (i_seam, &seam) in border.seams.iter().enumerate() {
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

    pub fn iter_seams(&self) -> impl Iterator<Item = &Seam> + Clone {
        self.iter_borders().flat_map(|border| border.seams.iter())
    }

    pub fn iter_seam_indices(&self) -> impl Iterator<Item = &SeamIndex> + Clone {
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

    pub fn material_left_of(&self, seam: &Seam) -> Material {
        self.material_map[seam.start.left_pixel]
    }

    /// Not every seam has a region on the right, it can be empty space
    pub fn right_of(&self, seam: &Seam) -> Option<RegionKey> {
        assert!(seam.is_atom());
        let seam_index = self.seam_indices.get(&seam.reversed().start)?;
        Some(seam_index.region_key)
    }

    pub fn material_right_of(&self, seam: &Seam) -> Option<Material> {
        self.material_map.get(seam.start.right_pixel()).cloned()
    }

    /// Only fails if seam is not self.contains_seam(seam)
    pub fn next_seam(&self, seam: &Seam) -> &Seam {
        let seam_index = self.seam_indices[&seam.start];
        let border = &self.regions[&seam_index.region_key].boundary.borders[seam_index.i_border];
        &border.seams[(seam_index.i_seam + seam.len) % border.seams.len()]
    }

    /// Only fails if seam is not self.contains_seam(seam)
    pub fn previous_seam(&self, seam: &Seam) -> &Seam {
        let seam_index = self.seam_indices[&seam.start];
        let border = &self.regions[&seam_index.region_key].boundary.borders[seam_index.i_border];
        &border.seams[(seam_index.i_seam - 1 + border.seams.len()) % border.seams.len()]
    }

    pub fn seam_materials(&self, seam: &Seam) -> SeamMaterials {
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
    ) -> impl Iterator<Item = &Seam> + Clone {
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

    pub fn iter_region_interior<'a>(
        &'a self,
        region_key: RegionKey,
    ) -> impl Iterator<Item = Pixel> + Clone + 'a {
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
                for seam in &border.seams {
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

/// Split a side cycle into segments of constant value.
pub fn split_cycle_into_seams<T: Eq + Copy + Debug>(cycle: &Vec<(Side, T)>) -> Vec<Seam> {
    assert!(!cycle.is_empty());

    // List pairs (side, value), (side', value') where value != value'
    let steps: Vec<((Side, T), (Side, T))> = cycle
        .iter()
        .copied()
        .circular_tuple_windows()
        .filter(|((_, lhs_value), (_, rhs_value))| lhs_value != rhs_value)
        .collect();

    if steps.is_empty() {
        // TODO: Reverse cycle should give same seam
        vec![Seam::new_atom(
            cycle.first().unwrap().0,
            cycle.last().unwrap().0,
        )]
    } else {
        steps
            .iter()
            .circular_tuple_windows()
            .map(|(lhs_step, rhs_step)| {
                let (start_side, start_value) = lhs_step.1;
                let (stop_side, stop_value) = rhs_step.0;
                assert_eq!(start_value, stop_value);
                Seam::new_atom(start_side, stop_side)
            })
            .collect()
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
