use crate::{
    cycle_segments::{CycleSegment, CycleSegments},
    field::RgbaField,
    material::Material,
    math::{
        pixel::{Corner, Pixel, Side},
        point::Point,
        rect::Rect,
    },
    new_regions::{BoundaryCycles, ConnectedCycleGroups, Cycle, CycleGroup, CycleMinSide, Sides},
    pixmap::{MaterialMap, Pixmap},
    regions::area_left_of_boundary,
    utils::{UndirectedEdge, UndirectedGraph},
};
use ahash::{HashMap, HashSet};
use itertools::Itertools;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    ops::{
        Bound::{Excluded, Unbounded},
        Index, IndexMut,
    },
    path::Path,
    sync::atomic::{AtomicI64, Ordering},
};

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
        // If other is a non-atomic seam it's a much harder problem
        assert!(!self.is_loop());
        &self.translated(offset) == other
    }
}

impl Display for Seam {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Seam({} -> {}, atoms: {})",
            self.start, self.stop, self.atoms
        )
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
pub type RegionKey = Side;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Border {
    /// Vec of (side, right region key), first side is minimum side of cycle,
    /// contains at least one item.
    pub sides: Vec<Side>,

    // /// Start side of first seam is first side in cycle, contains at least one item
    // pub seams: Vec<Seam>,
    pub cycle_segments: CycleSegments,

    pub is_outer: bool,
}

impl Border {
    pub fn from_cycle(cycle: &Cycle, boundary_cycles: &BoundaryCycles) -> Self {
        let cycle_reverse_cycles = cycle
            .sides
            .iter()
            .map(|side| boundary_cycles.side_to_cycle.get(&side.reversed()));
        let cycle_segments = CycleSegments::constant_segments_from_iter(cycle_reverse_cycles);

        Self {
            sides: cycle.sides.clone(),
            cycle_segments,
            is_outer: cycle.is_outer(),
        }
    }

    pub fn sides(&self) -> &[Side] {
        &self.sides
    }

    pub fn min_side(&self) -> CycleMinSide {
        self.sides[0]
    }

    pub fn iter_sides(
        &self,
    ) -> impl DoubleEndedIterator<Item = Side> + ExactSizeIterator + Clone + use<'_> {
        self.sides.iter().copied()
    }

    /// self + offset == other
    pub fn translated_eq(&self, offset: Point<i64>, other: &Self) -> bool {
        if self.sides.len() != other.sides.len() {
            return false;
        }

        self.sides
            .iter()
            .zip(&other.sides)
            .all(|(&self_side, &other_side)| self_side + offset == other_side)
    }

    /// Number of atomic seams in the border
    pub fn seams_len(&self) -> usize {
        self.cycle_segments.len()
    }

    fn seam_from_segment(&self, segment: CycleSegment, atoms: usize) -> Seam {
        Seam {
            start: self.sides[segment.first()],
            stop: self.sides[segment.last()],
            atoms,
        }
    }

    pub fn atomic_seam(&self, i: usize) -> Seam {
        self.seam_from_segment(self.cycle_segments.atomic_segment(i), 1)
    }

    pub fn seam(&self, start: usize, atoms: usize) -> Seam {
        self.seam_from_segment(self.cycle_segments.segment(start, atoms), atoms)
    }

    pub fn seam_sides(
        &self,
        start: usize,
        atoms: usize,
    ) -> impl DoubleEndedIterator<Item = Side> + Clone + use<'_> {
        self.cycle_segments
            .segment(start, atoms)
            .iter()
            .map(|i| self.sides[i])
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
            .map(|segment| self.seam_from_segment(segment, 1))
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

        Some(Seam::new_with_len(start_seam.start, stop_seam.stop, atoms))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Boundary {
    pub borders: Vec<Border>,

    pub interior_bounds: Rect<i64>,
}

impl Boundary {
    pub fn new(borders: Vec<Border>, interior_bounds: Rect<i64>) -> Self {
        assert!(!borders.is_empty());
        assert!(borders[0].is_outer);
        Self {
            borders,
            interior_bounds,
        }
    }

    pub fn outer_border(&self) -> &Border {
        self.borders.first().unwrap()
    }

    pub fn holes(&self) -> &[Border] {
        &self.borders[1..]
    }

    pub fn top_left_interior_pixel(&self) -> Pixel {
        let first_border = &self.borders[0];
        assert!(first_border.is_outer);
        first_border.sides[0].left_pixel
    }

    pub fn iter_sides(&self) -> impl Iterator<Item = Side> + Clone + use<'_> {
        self.borders.iter().flat_map(|border| border.iter_sides())
    }

    /// Compute the interior area (area left of the boundary). Prefer using `Topology::region_map`
    /// if available.
    pub fn interior_area(&self) -> Vec<Pixel> {
        area_left_of_boundary(self.iter_sides())
    }
}

/// The boundary is counterclockwise. Is mutable so material can be changed.
#[derive(Debug, Clone)]
pub struct Region {
    /// First item is outer Border
    pub boundary: Boundary,

    pub material: Material,

    pub modified_time: ModificationTime,
}

impl Region {
    fn from_cycle_group(
        cycle_group: &CycleGroup,
        material_map: &MaterialMap,
        borders: &mut HashMap<CycleMinSide, Border>,
    ) -> Self {
        let boundary_borders = cycle_group
            .cycle_min_sides
            .iter()
            .map(|cycle_min_side| borders.remove(cycle_min_side).unwrap())
            .collect();
        let boundary = Boundary::new(boundary_borders, cycle_group.bounds);

        Self {
            material: material_map
                .get(boundary.top_left_interior_pixel())
                .unwrap(),
            boundary,
            modified_time: modification_time_counter(),
        }
    }

    pub fn iter_seams(&self) -> impl Iterator<Item = Seam> + Clone + '_ {
        self.boundary
            .borders
            .iter()
            .flat_map(|border| border.atomic_seams())
    }

    pub fn top_left_interior_pixel(&self) -> Pixel {
        self.boundary.top_left_interior_pixel()
    }

    pub fn key(&self) -> RegionKey {
        self.boundary.outer_border().min_side()
    }

    pub fn bounds(&self) -> Rect<i64> {
        self.boundary.interior_bounds
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

#[derive(Debug, Clone)]
pub struct Topology {
    // TODO: Make Vec<Region>
    pub regions: BTreeMap<RegionKey, Region>,

    pub cycle_groups: ConnectedCycleGroups,

    pub boundary_cycles: BoundaryCycles,

    /// Maps the start side of a seam to (region key, border index, seam index)
    pub seam_indices: BTreeMap<Side, SeamIndex>,

    bounds: Rect<i64>,

    modifications: BTreeMap<ModificationTime, RegionKey>,
}

impl Topology {
    pub fn bounding_rect(&self) -> Rect<i64> {
        self.bounds
    }

    pub fn modifications(&self) -> &BTreeMap<ModificationTime, RegionKey> {
        &self.modifications
    }

    #[inline(never)]
    pub fn from_boundary_cycles(
        boundary_cycles: BoundaryCycles,
        material_map: &MaterialMap,
    ) -> Self {
        let _tracy_span = tracy_client::span!("Topology::from_boundary_cycles");

        let cycle_groups = ConnectedCycleGroups::from_cycles(&boundary_cycles);

        // Create a Border for each cycle if we can't recycle one
        let mut borders = HashMap::default();
        for (&cycle_min_side, cycle) in &boundary_cycles.cycles {
            borders.insert(cycle_min_side, Border::from_cycle(cycle, &boundary_cycles));
        }

        // Create Regions from cycle_groups
        let mut regions: BTreeMap<RegionKey, Region> = BTreeMap::new();
        let mut seam_indices: BTreeMap<Side, SeamIndex> = BTreeMap::new();
        let mut modifications: BTreeMap<ModificationTime, RegionKey> = BTreeMap::new();

        {
            let _tracy_span = tracy_client::span!("build regions");

            for cycle_group in cycle_groups.groups() {
                let region_key = cycle_group.outer_cycle_min_side();
                let region = Region::from_cycle_group(cycle_group, material_map, &mut borders);

                // Update seam indices
                for (i_border, border) in region.boundary.borders.iter().enumerate() {
                    for (i_seam, seam) in border.atomic_seams().enumerate() {
                        let seam_index = SeamIndex::new(region_key, i_border, i_seam);
                        seam_indices.insert(seam.start, seam_index);
                    }
                }

                modifications.insert(region.modified_time, region_key);
                regions.insert(region_key, region);
            }
        }

        Self {
            bounds: boundary_cycles.bounds,
            cycle_groups,
            boundary_cycles,
            regions,
            seam_indices,
            modifications,
        }
    }

    #[inline(never)]
    pub fn new(material_map: &MaterialMap) -> Self {
        let boundary_sides = Sides::boundary_sides(material_map);
        let boundary_cycles = BoundaryCycles::new(&boundary_sides);
        Self::from_boundary_cycles(boundary_cycles, material_map)
    }

    pub fn region_map(&self) -> Pixmap<RegionKey> {
        self.cycle_groups.region_map(&self.boundary_cycles)
    }

    pub fn iter_regions(&self) -> impl Iterator<Item = (RegionKey, &Region)> + Clone {
        self.regions.iter().map(|(&key, region)| (key, region))
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

    pub fn iter_seams(&self) -> impl Iterator<Item = Seam> + Clone + use<'_> {
        self.iter_borders().flat_map(|border| border.atomic_seams())
    }

    pub fn iter_seam_indices(&self) -> impl Iterator<Item = SeamIndex> + Clone + use<'_> {
        self.seam_indices.values().copied()
    }

    pub fn iter_region_keys<'a>(&'a self) -> impl Iterator<Item = RegionKey> + Clone + 'a {
        self.regions.keys().copied()
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

    /// Returns the atomic seam with the given key.
    pub fn atomic_seam(&self, key: SeamIndex) -> Seam {
        self.regions[&key.region_key].boundary.borders[key.i_border].atomic_seam(key.i_seam)
    }

    /// Returns the border of a given seam
    /// Only fails if seam is not self.contains_seam(seam)
    pub fn seam_border(&self, seam: Seam) -> BorderKey {
        // TODO: Use `seam_index`
        let seam_index = &self.seam_indices[&seam.start];
        seam_index.border_index()
    }

    /// Iterate over the sides of the given seam
    pub fn seam_sides(
        &self,
        seam_key: SeamIndex,
    ) -> impl DoubleEndedIterator<Item = Side> + use<'_> {
        let region = &self.regions[&seam_key.region_key];
        let border = &region.boundary.borders[seam_key.i_border];
        border.seam_sides(seam_key.i_seam, 1)
    }

    /// Return the region key on the left side of a given seam
    /// Only fails if seam is not self.contains_seam(seam)
    pub fn left_of(&self, seam: Seam) -> RegionKey {
        // TODO: Use `seam_index`
        let seam_index = &self.seam_indices[&seam.start];
        seam_index.region_key
    }

    pub fn material_at(&self, pixel: Point<i64>) -> Option<Material> {
        let region = self.region_at(pixel)?;
        Some(region.material)
    }

    pub fn material_left_of(&self, seam: Seam) -> Material {
        self.material_at(seam.start.left_pixel).unwrap()
    }

    /// Not every seam has a region on the right, it can be empty space
    pub fn right_of(&self, seam: Seam) -> Option<RegionKey> {
        assert!(seam.is_atom());
        // TODO: Use `seam_index`
        let seam_index = self.seam_indices.get(&seam.atom_reversed().start)?;
        Some(seam_index.region_key)
    }

    pub fn material_right_of(&self, seam: Seam) -> Option<Material> {
        self.material_at(seam.start.right_pixel())
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

    // /// Returns all regions that touch any of the given `pixels`
    // pub fn touched_regions(&self, pixels: impl IntoIterator<Item = Pixel>) -> BTreeSet<RegionKey> {
    //     pixels
    //         .into_iter()
    //         .flat_map(|pixel| pixel.touching())
    //         .flat_map(|touched| self.region_map.get(touched))
    //         .collect()
    // }

    /// Is the right side of the seam void? The left side is always a proper region.
    pub fn touches_void(&self, seam: Seam) -> bool {
        self.right_of(seam).is_none()
    }

    pub fn seams_equivalent(&self, lhs: Seam, rhs: Seam) -> bool {
        lhs.is_loop() && rhs.is_loop() && self.seam_border(lhs) == self.seam_border(rhs)
    }

    // TODO: Slow
    pub fn iter_region_interior<'a>(
        &'a self,
        region_key: RegionKey,
    ) -> impl Iterator<Item = Pixel> + Clone + 'a {
        let region = &self.regions[&region_key];
        region.boundary.interior_area().into_iter()
    }

    pub fn region_key_at(&self, pixel: Pixel) -> Option<RegionKey> {
        self.cycle_groups
            .outer_cycle_of_region_at(&self.boundary_cycles, pixel)
    }

    pub fn region_at(&self, pixel: Pixel) -> Option<&Region> {
        let region_key = self.region_key_at(pixel)?;
        Some(&self.regions[&region_key])
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

    /// Try to set the material of `region_key` to `material`, returns true if successful.
    pub fn try_set_region_material(&mut self, region_key: RegionKey, material: Material) -> bool {
        let region = &self.regions.get(&region_key).unwrap();
        if region.material == material {
            return true;
        }

        // Try to update the topology to the changed material map
        let can_set = region
            .iter_seams()
            .all(|seam| self.material_right_of(seam) != Some(material));

        if !can_set {
            return false;
        }

        let region = self.regions.get_mut(&region_key).unwrap();
        self.modifications.remove(&region.modified_time);
        let modified_time = modification_time_counter();
        // info!(
        //     "increasing modified_time of Region {} from {} to {}",
        //     region_key, region.modified_time, modified_time
        // );
        region.modified_time = modified_time;
        region.material = material;
        self.modifications.insert(modified_time, region_key);

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
        Ok(Topology::new(&material_map))
    }

    pub fn regions_by_material(
        &self,
        material: Material,
    ) -> impl Iterator<Item = (RegionKey, &Region)> {
        self.iter_regions()
            .filter(move |(_, region)| region.material == material)
    }

    pub fn modifications_after(
        &self,
        mtime: ModificationTime,
    ) -> impl Iterator<Item = (ModificationTime, RegionKey)> + use<'_> {
        self.modifications
            .range((Excluded(mtime), Unbounded))
            .map(|(&mtime, &region_key)| (mtime, region_key))
    }

    pub fn first_modification_after(
        &self,
        mtime: ModificationTime,
    ) -> Option<(ModificationTime, RegionKey)> {
        self.modifications_after(mtime).next()
    }

    /// Draw the given (pixel, material) pairs. Equivalent to drawing to the underlying MaterialMap
    /// and recreating the Topology.
    #[inline(never)]
    pub fn update(
        &mut self,
        material_map: &MaterialMap,
        pixels: impl Iterator<Item = Pixel> + Clone,
    ) {
        let _tracy_span = tracy_client::span!("Topology::draw");

        let draw_bounds = Rect::index_bounds(pixels.clone());
        let padded_draw_bounds = draw_bounds.padded(1);

        let touched_cycles = self.boundary_cycles.update(material_map, pixels);

        let mut borders = HashMap::default();

        // Discard regions that potentially touch the draw pixels with their seam indices.
        self.regions.retain(|_, region| {
            let discard = region.bounds().intersects(padded_draw_bounds);
            if discard {
                // Remove seam indices of region
                for seam in region.iter_seams() {
                    self.seam_indices.remove(&seam.start);
                }

                // Remove from modifications
                self.modifications.remove(&region.modified_time);

                // Try to recycle borders of discarded regions, that are touched by the draw pixels.
                for border in region.boundary.borders.drain(..) {
                    let cycle_min_side = border.min_side();
                    if !touched_cycles.contains(&cycle_min_side) {
                        borders.insert(cycle_min_side, border);
                    }
                }
            }
            !discard
        });

        // TODO: This is currently the biggest performance bottleneck. We should only recompute
        //   groups where necessary.
        self.cycle_groups = ConnectedCycleGroups::from_cycles(&self.boundary_cycles);

        // Create a new Border for each cycle if it wasn't recycled
        for cycle_min_side in touched_cycles {
            let cycle = &self.boundary_cycles.cycles[&cycle_min_side];
            borders
                .entry(cycle_min_side)
                .or_insert_with(|| Border::from_cycle(cycle, &self.boundary_cycles));
        }

        // Create Regions from cycle_groups
        for cycle_group in self.cycle_groups.groups() {
            let region_key = cycle_group.outer_cycle_min_side();

            if self
                .regions
                .contains_key(&cycle_group.outer_cycle_min_side())
            {
                // We were able to keep this region
                continue;
            }

            let region = Region::from_cycle_group(cycle_group, material_map, &mut borders);

            // Update seam indices
            for (i_border, border) in region.boundary.borders.iter().enumerate() {
                for (i_seam, seam) in border.atomic_seams().enumerate() {
                    let seam_index = SeamIndex::new(region_key, i_border, i_seam);
                    self.seam_indices.insert(seam.start, seam_index);
                }
            }

            self.modifications.insert(region.modified_time, region_key);
            self.regions.insert(region_key, region);
        }

        // For debugging, reset all modified times
        // self.modifications.clear();
        // for (&region_key, region) in &mut self.regions {
        //     region.modified_time = modification_time_counter();
        //     self.modifications.insert(region.modified_time, region_key);
        // }

        // For debugging, fully recreate Topology
        // *self = Topology::new(material_map);
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

pub type ModificationTime = i64;

pub fn modification_time_counter() -> ModificationTime {
    static MTIME: AtomicI64 = AtomicI64::new(0);
    MTIME.fetch_add(1, Ordering::Relaxed)
}

// TODO: It might make more sense to mask a Morphism, so we can panic if a hidden element is
// assigned.
pub struct MaskedTopology<'a> {
    pub inner: &'a Topology,
    pub hidden: Option<&'a HashSet<RegionKey>>,
}

impl<'a> MaskedTopology<'a> {
    pub fn new(topology: &'a Topology, hidden: &'a HashSet<RegionKey>) -> Self {
        Self {
            inner: topology,
            hidden: Some(hidden),
        }
    }

    pub fn whole(topology: &'a Topology) -> Self {
        Self {
            inner: topology,
            hidden: None,
        }
    }

    pub fn is_hidden(&self, region_key: RegionKey) -> bool {
        match self.hidden {
            None => false,
            Some(hidden) => hidden.contains(&region_key),
        }
    }

    pub fn visible_regions(
        &'a self,
    ) -> impl Iterator<Item = (RegionKey, &'a Region)> + Clone + use<'a> {
        self.inner
            .iter_regions()
            .filter(|(region_key, _)| !self.is_hidden(*region_key))
    }

    pub fn visible_region_keys(&'a self) -> impl Iterator<Item = RegionKey> + Clone + use<'a> {
        self.visible_regions().map(|(key, _)| key)
    }
}

impl<'a> From<&'a Topology> for MaskedTopology<'a> {
    fn from(topology: &'a Topology) -> Self {
        MaskedTopology::whole(topology)
    }
}

#[derive(Debug, Clone, Default)]
pub struct TopologyStatistics {
    pub material_counts: HashMap<Material, usize>,
}

impl TopologyStatistics {
    pub fn new(topology: &MaskedTopology) -> Self {
        let mut statistics = TopologyStatistics::default();
        for (_, region) in topology.visible_regions() {
            *statistics
                .material_counts
                .entry(region.material)
                .or_default() += 1;
        }
        statistics
    }
}

#[cfg(test)]
pub mod test {
    use crate::{
        material::Material,
        math::{point::Point, rect::Rect},
        pixmap::MaterialMap,
        topology::{SeamIndex, Topology},
        utils::{KeyValueItertools, UndirectedEdge, UndirectedGraph},
    };
    use itertools::Itertools;
    use std::{collections::BTreeSet, mem::swap};

    fn load_topology(filename: &str) -> Topology {
        let path = format!("test_resources/topology/{filename}");
        Topology::load(path).unwrap()
    }

    fn rgb_edges_from<const N: usize>(
        rgb_edges: [(Material, Option<Material>); N],
    ) -> UndirectedGraph<Option<Material>> {
        rgb_edges
            .into_iter()
            .map(|(a, b)| UndirectedEdge::new(Some(a), b))
            .collect()
    }

    fn check_structure(topology: &Topology) {
        // Check if region_map is consistent with Region list
        let region_map = topology.region_map();

        for (&region_key, region) in &topology.regions {
            let area_from_region_map: BTreeSet<_> = region_map
                .iter()
                .filter_key_by_value(|&key| key == region_key)
                .collect();

            let area_from_boundary: BTreeSet<_> =
                region.boundary.interior_area().into_iter().collect();

            assert_eq!(area_from_region_map, area_from_boundary);
        }

        // Make sure seam_index is correct
        for (&region_key, region) in &topology.regions {
            for (i_border, border) in region.boundary.borders.iter().enumerate() {
                for (i_seam, seam) in border.atomic_seams().enumerate() {
                    assert_eq!(
                        SeamIndex::new(region_key, i_border, i_seam),
                        topology.seam_indices[&seam.start]
                    );
                }
            }
        }

        // Check if seams are proper
        for seam_key in topology.iter_seam_indices() {
            // Make sure left and right side material are constant over the seam
            let _left_region_key = topology
                .seam_sides(seam_key)
                .map(|side| region_map.get(side.left_pixel))
                .dedup()
                .exactly_one()
                .ok()
                .unwrap();

            let right_region_key = topology
                .seam_sides(seam_key)
                .map(|side| region_map.get(side.right_pixel()))
                .dedup()
                .exactly_one()
                .ok()
                .unwrap();

            // If right of seam is not None, the reverse of the seam must be in the Topology
            if right_region_key.is_some() {
                let seam = topology.atomic_seam(seam_key);
                assert!(topology.contains_seam(seam.atom_reversed()));
            }
        }
    }

    pub fn check_topology(filename: &str, expected_seam_graph: &UndirectedGraph<Option<Material>>) {
        let topology = load_topology(filename);
        check_structure(&topology);

        let seam_graph = topology.material_seam_graph();
        assert_eq!(&seam_graph, expected_seam_graph);
    }

    #[test]
    fn topology_1a() {
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
            (Material::TRANSPARENT, None),
            (Material::RED, Some(Material::TRANSPARENT)),
        ]);
        check_topology("1a.png", &expected_edges);
    }

    #[test]
    fn topology_2a() {
        let expected_edges = rgb_edges_from([
            (Material::TRANSPARENT, None),
            (Material::RED, Some(Material::BLUE)),
            (Material::RED, Some(Material::TRANSPARENT)),
            (Material::BLUE, Some(Material::TRANSPARENT)),
        ]);
        check_topology("2a.png", &expected_edges);
    }

    #[test]
    fn topology_2b() {
        let expected_edges = rgb_edges_from([
            (Material::TRANSPARENT, None),
            (Material::RED, Some(Material::BLUE)),
            (Material::RED, Some(Material::TRANSPARENT)),
        ]);
        check_topology("2b.png", &expected_edges);
    }

    #[test]
    fn image_2c() {
        let topology = load_topology("2c.png");
        assert_eq!(topology.regions.len(), 3);
    }

    #[test]
    fn topology_3a() {
        let expected_edges = rgb_edges_from([
            (Material::TRANSPARENT, None),
            (Material::RED, Some(Material::GREEN)),
            (Material::RED, Some(Material::BLUE)),
            (Material::RED, Some(Material::TRANSPARENT)),
        ]);
        check_topology("3a.png", &expected_edges);
    }

    #[test]
    fn topology_3b() {
        let expected_edges = rgb_edges_from([
            (Material::RED, None),
            (Material::RED, Some(Material::GREEN)),
            (Material::GREEN, Some(Material::BLUE)),
        ]);
        check_topology("3b.png", &expected_edges);
    }

    #[test]
    fn topology_3c() {
        let expected_edges = rgb_edges_from([
            (Material::TRANSPARENT, None),
            (Material::RED, Some(Material::YELLOW)),
            (Material::RED, Some(Material::BLUE)),
            (Material::YELLOW, Some(Material::BLUE)),
            (Material::RED, Some(Material::TRANSPARENT)),
        ]);
        check_topology("3c.png", &expected_edges);
    }

    #[test]
    fn topology_4a() {
        let expected_edges = rgb_edges_from([
            (Material::TRANSPARENT, None),
            (Material::RED, Some(Material::BLUE)),
            (Material::BLUE, Some(Material::GREEN)),
            (Material::GREEN, Some(Material::CYAN)),
            (Material::BLUE, Some(Material::CYAN)),
            (Material::RED, Some(Material::CYAN)),
            (Material::RED, Some(Material::TRANSPARENT)),
        ]);
        check_topology("4a.png", &expected_edges);
    }

    // fn check_draw_topology
    fn check_draw_topology(from_material_map: &MaterialMap, to_material_map: &MaterialMap) {
        assert_eq!(
            from_material_map.bounding_rect(),
            to_material_map.bounding_rect()
        );

        let modified_pixels: Vec<_> = from_material_map
            .field
            .indices()
            .filter_map(|pixel| {
                let from_material = from_material_map.get(pixel);
                let to_material = to_material_map.get(pixel);
                (from_material != to_material).then_some(pixel)
            })
            .collect();

        let from_topology = Topology::new(&from_material_map);
        let to_topology = Topology::new(&to_material_map);

        let mut drawn_topology = from_topology.clone();
        drawn_topology.update(&to_material_map, modified_pixels.iter().copied());

        check_structure(&drawn_topology);

        assert_eq!(to_topology.region_map(), drawn_topology.region_map());
        assert_eq!(to_topology, drawn_topology);
    }

    /// Uses transparent as None color
    fn check_draw_topology_from_files(from_filename: &str, to_filename: &str) {
        let folder = "test_resources/topology/draw";
        let from_material_map = MaterialMap::load(format!("{folder}/{from_filename}"))
            .unwrap()
            .without(Material::TRANSPARENT);
        let to_material_map = MaterialMap::load(format!("{folder}/{to_filename}"))
            .unwrap()
            .without(Material::TRANSPARENT);

        check_draw_topology(&from_material_map, &to_material_map);
    }

    #[test]
    fn draw_small0_to_small1() {
        check_draw_topology_from_files("small0.png", "small1.png");
    }

    #[test]
    fn draw_topology_a0_to_a1() {
        check_draw_topology_from_files("a0.png", "a1.png");
    }

    #[test]
    fn draw_topology_a1_to_a2() {
        check_draw_topology_from_files("a1.png", "a2.png");
    }

    #[test]
    fn draw_topology_a2_to_a3() {
        check_draw_topology_from_files("a2.png", "a3.png");
    }

    #[test]
    fn draw_topology_b0_to_b1() {
        check_draw_topology_from_files("b0.png", "b1.png");
    }

    #[test]
    fn draw_topology_b1_to_b2() {
        check_draw_topology_from_files("b1.png", "b2.png");
    }

    #[test]
    fn draw_topology_c0_to_c1() {
        // Approach does not work for this case.
        // Alternative approach: Calculate PreRegion { borders } for each touched Region, then
        // calculate seams in context of Topology.
        check_draw_topology_from_files("c0.png", "c1.png");
    }

    #[test]
    fn draw_topology_hole0_to_hole1() {
        check_draw_topology_from_files("hole0.png", "hole1.png");
    }

    #[test]
    fn draw_topology_bridge0_to_bridge1() {
        check_draw_topology_from_files("bridge0.png", "bridge1.png");
    }

    #[test]
    fn draw_topology_fail_a0_to_fail_a1() {
        check_draw_topology_from_files("fail_a0.png", "fail_a1.png");
    }

    #[test]
    fn draw_topology_clear0_to_clear1() {
        check_draw_topology_from_files("clear0.png", "clear1.png");
    }

    #[test]
    fn draw_topology_merge0_to_merge1() {
        check_draw_topology_from_files("merge0.png", "merge1.png");
    }

    #[test]
    fn draw_topology_merge1_to_merge0() {
        check_draw_topology_from_files("merge1.png", "merge0.png");
    }

    #[test]
    fn draw_topology_color1_to_color1() {
        check_draw_topology_from_files("color0.png", "color1.png");
    }

    #[test]
    fn draw_random() {
        let materials = [
            Material::RED,
            Material::BLUE,
            Material::GREEN,
            Material::WHITE,
        ];
        let size = 16;

        let mut rng = fastrand::Rng::with_seed(42);

        let mut material_map = MaterialMap::nones(Rect::low_size(Point(0, 0), Point(size, size)));

        // Run in release mode for higher number of tries
        for _ in 0..100 {
            let mut drawn_material_map = material_map.clone();
            // Randomly draw 32 pixels with a small choice of colors
            for _ in 0..32 {
                let pixel = Point(rng.i64(0..size), rng.i64(0..size));
                let material = rng.choice(materials).unwrap();
                drawn_material_map.set(pixel, material);
            }

            check_draw_topology(&material_map, &drawn_material_map);
            swap(&mut material_map, &mut drawn_material_map);
        }
    }
}
