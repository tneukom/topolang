/// Unionfind crates
/// https://crates.io/crates/union-find
/// https://crates.io/crates/disjoint-sets
///
/// https://crates.io/crates/partitions
/// Has clear function
///
/// https://docs.rs/petgraph/latest/src/petgraph/unionfind.rs.html#16-27
use std::time::Instant;
use std::{collections::BTreeMap, rc::Rc};

use crate::{
    field::Field,
    math::pixel::SideName,
    pixmap::{iter_tile_boundary_sides, Neighborhood, Pixmap, Tile},
    union_find::UnionFind,
};

pub struct CompactLabels {
    remap: Vec<usize>,
    counter: usize,
}

/// Maps the given labels to [0,...,n] where n is the number of distinct labels.
impl CompactLabels {
    #[inline(never)]
    pub fn new(max_label: usize) -> Self {
        Self {
            remap: vec![usize::MAX; max_label],
            counter: 0,
        }
    }

    #[inline(never)]
    pub fn clear(&mut self) {
        self.counter = 0;
        for i in 0..self.remap.len() {
            self.remap[i] = usize::MAX;
        }
    }

    pub fn remap(&mut self, label: usize) -> usize {
        if self.remap[label] == usize::MAX {
            self.remap[label] = self.counter;
            self.counter += 1;
        }
        self.remap[label]
    }

    #[inline(never)]
    pub fn compact<'a>(&mut self, labels: impl IntoIterator<Item = &'a mut usize>) {
        for label in labels {
            *label = self.remap(*label);
        }
    }

    #[inline(never)]
    pub fn compact_masked<'a>(&mut self, labels: impl IntoIterator<Item = &'a mut Option<usize>>) {
        for label in labels {
            if let Some(label) = label {
                *label = self.remap(*label);
            }
        }
    }
}

trait NeighborsFn<T: Copy> {
    fn call<const N: usize>(&mut self, field: &Field<T>, center: usize, neighbors: [usize; N]);

    fn for_each(&mut self, field: &Field<T>) {
        let width = field.width() as usize;
        let height = field.height() as usize;

        // Top pixels
        for x in 1..width {
            let center = x;
            self.call(field, center, [center - 1]);
        }

        // Left pixels
        for y in 1..height {
            let center = y * width;
            self.call(field, center, [center - width])
        }

        // Interior pixels
        for y in 1..height {
            for x in 1..width {
                let center = y * width + x;
                self.call(
                    field,
                    center,
                    [center - 1, center - width, center - width - 1],
                );
            }
        }
    }
}

struct BuildUnionFind {
    union_find: UnionFind,
}

impl<T: Eq + Copy> NeighborsFn<T> for BuildUnionFind {
    fn call<const N: usize>(&mut self, field: &Field<T>, center: usize, neighbors: [usize; N]) {
        let center_value = field.as_slice()[center];
        for neighbor in neighbors {
            let neighbor_value = field.as_slice()[neighbor];
            if neighbor_value == center_value {
                self.union_find.union(center, neighbor);
            }
        }
    }
}

#[inline(never)]
pub fn field_regions_fast<T: Copy + Eq>(field: &Field<T>) -> Field<usize> {
    let union_find = UnionFind::new(field.len());
    let mut build_union_find = BuildUnionFind { union_find };
    build_union_find.for_each(field);
    let labeling = build_union_find.union_find.into_roots();
    Field::from_linear(field.bounds(), labeling)
}

/// Returns connected components of `field`
#[inline(never)]
pub fn field_regions<T: Clone + Eq>(field: &Field<T>) -> Field<usize> {
    // TODO: Reuse UnionFind so we don't allocate each time
    let mut union_find = UnionFind::new(field.len());

    for (center, value) in field.enumerate() {
        for side in [SideName::TopLeft, SideName::Top, SideName::Left] {
            let neighbor = center.neighbor(side);
            if Some(value) == field.get(neighbor) {
                union_find.union(
                    field.linear_index(center).unwrap(),
                    field.linear_index(neighbor).unwrap(),
                );
            }
        }
    }

    let labeling = union_find.into_roots();
    Field::from_linear(field.bounds(), labeling)
}

#[inline(never)]
pub fn mask_field<T: Clone, S>(field: &Field<T>, mask: &Field<Option<S>>) -> Field<Option<T>> {
    let elems = field
        .iter()
        .zip(mask.iter())
        .map(|(el, mask_el)| mask_el.as_ref().map(|_| el.clone()))
        .collect();
    Field::from_linear(field.bounds(), elems)
}

#[inline(never)]
pub fn tile_regions<T: Copy + Eq>(tile: &Tile<T>) -> (Tile<usize>, usize) {
    let mut region_field = field_regions_fast(&tile.field);
    let mut region_field = mask_field(&mut region_field, &tile.field);

    // TODO: Reuse CompactLabels so we don't allocate each time
    let mut compact_labels = CompactLabels::new(region_field.len());
    compact_labels.compact_masked(region_field.iter_mut());
    (Tile::from_field(region_field), compact_labels.counter)
}

pub fn pixmap_regions<T: Copy + Eq>(color_map: &Pixmap<T>) -> Pixmap<usize> {
    // Map pixmap tile wise (not regarding interface between tiles)
    let mut n_total_regions = 0;
    let region_map_tiles: BTreeMap<_, _> = color_map
        .tiles
        .iter()
        .map(|(&tile_index, tile)| {
            let (mut region_tile, n_regions) = tile_regions(tile);
            // Offset region ids
            for (_, region) in region_tile.iter_mut() {
                *region += n_total_regions;
            }
            n_total_regions += n_regions;
            (tile_index, Rc::new(region_tile))
        })
        .collect();
    let mut region_map = Pixmap {
        tiles: region_map_tiles,
    };

    // Build UnionFind
    let now = Instant::now();
    let mut union_find = UnionFind::new(n_total_regions);
    for &tile_index in color_map.tiles.keys() {
        let color_neighborhood = Neighborhood::from_pixmap(color_map, tile_index);
        let region_neighborhood = Neighborhood::from_pixmap(&region_map, tile_index);

        // TODO: iter_tile_boundary_sides uses a global variable, therefore atomic compare
        for side in iter_tile_boundary_sides(tile_index) {
            let color_neighbors = color_neighborhood.side_neighbors(side);
            if let (Some(left_color), Some(right_color)) =
                (color_neighbors.left, color_neighbors.right)
            {
                if left_color == right_color {
                    let region_neighbors = region_neighborhood.side_neighbors(side);
                    let &left_region = region_neighbors.left.unwrap();
                    let &right_region = region_neighbors.right.unwrap();
                    union_find.union(left_region, right_region);
                }
            }
        }
    }

    let mut roots = union_find.into_roots();
    let mut compact_labels = CompactLabels::new(n_total_regions);
    compact_labels.compact(&mut roots);

    for (_, region_id) in region_map.iter_mut() {
        *region_id = roots[*region_id]
    }
    println!("Merging elapsed = {:.3?}", now.elapsed());

    region_map
}

#[cfg(test)]
mod test {
    use std::cell::Cell;

    use crate::{
        field::Field,
        math::{generic::EuclidDivRem, pixel::Side, point::Point, rect::Rect, rgba8::Rgba8},
        pixmap::{split_index, MaterialMap, Pixmap, TILE_SIZE},
        regions::pixmap_regions,
        utils::IntoT,
    };

    pub fn pixel_touches_tile_boundary(index: Point<i64>) -> bool {
        let (_, pixel_index) = split_index(index);
        pixel_index.x == 0
            || pixel_index.x == TILE_SIZE - 1
            || pixel_index.y == 0
            || pixel_index.y == TILE_SIZE - 1
    }

    pub fn side_lies_on_tile_boundary(side: Side) -> bool {
        pixel_touches_tile_boundary(side.left_pixel)
    }

    fn generate_field(bounds: Rect<i64>) -> Field<Rgba8> {
        Field::from_map(bounds, |index| {
            let red = index.x.euclid_rem(256) as u8;
            let green = index.y.euclid_rem(256) as u8;
            Rgba8::new(red, green, 255, 255)
        })
    }

    fn field_bounds_cases() -> Vec<Rect<i64>> {
        return vec![
            Rect::low_high([0, 0], [1, 1]),
            Rect::low_high([-1, -1], [0, 0]),
            Rect::low_high([0, 0], [TILE_SIZE - 1, TILE_SIZE - 1]),
            Rect::low_high([0, 0], [TILE_SIZE + 1, TILE_SIZE - 1]),
            Rect::low_high([-391, 234], [519, 748]),
        ];
    }

    fn test_mark_tile_interface(name: &str) {
        let folder = "test_resources/regions/mark_tile_interface";

        let before: Pixmap<Rgba8> = Field::load(format!("{folder}/{name}_before.png"))
            .unwrap()
            .into();
        // Turn into Pixmap2<Cell<Rgba8>> so we can modify it
        let before_cell = before.into_map(|rgba| Cell::new(rgba));
        before_cell.iter_side_neighbors().for_each(|side| {
            if side.left == Some(&Cell::new(Rgba8::BLUE)) {
                let right = side.right.unwrap();
                if right.get() == Rgba8::TRANSPARENT {
                    right.set(Rgba8::RED);
                }
            }
        });

        let after = before_cell.into_map(|cell| cell.get());
        // after
        //     .to_field(Rgba8::ZERO)
        //     .save(format!("{folder}/{name}_out.png"))
        //     .unwrap();

        let expected: Pixmap<Rgba8> = Field::load(format!("{folder}/{name}_after.png"))
            .unwrap()
            .into();

        assert_eq!(after, expected);
    }

    #[test]
    fn mark_tile_interface_a() {
        test_mark_tile_interface("a")
    }

    #[test]
    fn mark_tile_interface_b() {
        test_mark_tile_interface("b")
    }

    #[test]
    fn convert_field() {
        for bounds in field_bounds_cases() {
            let field = generate_field(bounds);
            let pixmap = Pixmap::from_field(&field);
            assert_eq!(pixmap.bounding_rect(), bounds);

            for (index, value) in field.enumerate() {
                assert_eq!(Some(value), pixmap.get(index));
            }

            assert_eq!(pixmap.to_field(Rgba8::ZERO), field);
        }
    }

    #[test]
    fn pixmap2_regions() {
        let folder = "test_resources/regions";
        let color_map = Field::load(format!("{folder}/c.png"))
            .unwrap()
            .intot::<MaterialMap>();

        let region_map = pixmap_regions(&color_map);
        let region_field = region_map.to_field(255);
        let region_field_rgba = region_field.map(|id| Rgba8::new(*id as u8, 0, 0, 255));
        region_field_rgba
            .save(format!("{folder}/c_out.png"))
            .unwrap();
    }

    // #[test]
    // fn regions() {
    //     let folder = "test_resources/regions";
    //
    //     let color_map: Pixmap2<Rgba8> = Field::load(format!("{folder}/b.png")).unwrap().into();
    //
    //     let region_map = pixmap_regions2(&color_map);
    //
    //     // convert to Rgba8
    //     let region_map_rgba = region_map.map(|id| Rgba8::new(*id as u8, 0, 0, 255));
    //
    //     // region_map_rgba
    //     //     .to_field(Rgba8::ZERO)
    //     //     .save(format!("{folder}/b_out.png"))
    //     //     .unwrap();
    // }
}
