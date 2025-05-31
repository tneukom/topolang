use crate::{
    math::{
        pixel::{Pixel, Side, SideName},
        rect::Rect,
    },
    pixmap::MaterialMap,
    regions::{area_left_of_boundary, iter_pixmap_sides, split_boundary_into_cycles},
};
use itertools::Itertools;
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

type CycleMinSide = Side;

/// Given the minimal side of a cycle return if the cycle is an outer border of a region. Outer
/// borders are counterclockwise.
pub fn is_outer_border(min_side: Side) -> bool {
    min_side.name == SideName::Left
}

pub fn is_inner_border(min_side: Side) -> bool {
    !is_outer_border(min_side)
}

pub struct Cycle {
    pub sides: Vec<Side>,
    pub bounds: Rect<i64>,
}

impl Cycle {
    pub fn new(sides: Vec<Side>) -> Self {
        assert!(!sides.is_empty());
        for &side in &sides {
            debug_assert!(sides[0] <= side);
        }
        for (fst_side, snd_side) in sides.iter().circular_tuple_windows() {
            debug_assert_eq!(fst_side.stop_corner(), snd_side.start_corner());
        }

        let bounds = Rect::index_bounds(sides.iter().map(|side| side.left_pixel));
        Self { sides, bounds }
    }

    pub fn interior_pixel(&self) -> Pixel {
        self.sides[0].left_pixel
    }

    pub fn is_inner(&self) -> bool {
        is_inner_border(self.sides[0])
    }

    pub fn is_outer(&self) -> bool {
        is_outer_border(self.sides[0])
    }
}

pub struct BoundaryCycles {
    /// Mapping from the min side of each cycle to the cycle itself
    cycles: BTreeMap<CycleMinSide, Cycle>,

    /// Mapping from any side of a cycle to the min side of the cycle
    side_to_cycle: BTreeMap<Side, CycleMinSide>,
}

impl BoundaryCycles {
    pub fn from_material_map(material_map: &MaterialMap) -> Self {
        // Collect boundary sides of `material_map`
        let mut sides = BTreeSet::new();
        for (side, left, right) in iter_pixmap_sides(material_map) {
            if Some(left) != right {
                sides.insert(side);
            }
        }

        // Split boundary sides into cycles
        let cycles: BTreeMap<Side, Cycle> = split_boundary_into_cycles(sides)
            .into_iter()
            .map(|cycle_sides| (cycle_sides[0], Cycle::new(cycle_sides)))
            .collect();

        // `cycle_map` maps each side to the cycle index it belongs to.
        let mut side_to_cycle = BTreeMap::new();
        for (&first_side, cycle) in cycles.iter() {
            for &side in &cycle.sides {
                side_to_cycle.insert(side, first_side);
            }
        }

        Self {
            cycles,
            side_to_cycle,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CycleGroup {
    /// Min sides of the cycles belonging to this group
    pub cycle_min_sides: Vec<CycleMinSide>,
    pub bounds: Rect<i64>,
}

impl CycleGroup {
    pub fn new(outer_cycle_min_side: CycleMinSide, bounds: Rect<i64>) -> Self {
        assert!(is_outer_border(outer_cycle_min_side));
        Self {
            cycle_min_sides: vec![outer_cycle_min_side],
            bounds,
        }
    }

    pub fn add_inner_cycle(&mut self, inner_cycle_min_side: CycleMinSide) {
        assert!(is_inner_border(inner_cycle_min_side));
        self.cycle_min_sides.push(inner_cycle_min_side);
    }

    pub fn interior_pixel(&self) -> Pixel {
        self.cycle_min_sides[0].left_pixel
    }

    pub fn iter_sides<'a>(
        &'a self,
        boundary_cycles: &'a BoundaryCycles,
    ) -> impl Iterator<Item = Side> + Clone + use<'a> {
        self.iter_cycles(boundary_cycles)
            .flat_map(|cycle| &cycle.sides)
            .copied()
    }

    pub fn iter_cycles<'a>(
        &'a self,
        boundary_cycles: &'a BoundaryCycles,
    ) -> impl Iterator<Item = &'a Cycle> + Clone + use<'a> {
        self.cycle_min_sides
            .iter()
            .map(|cycle_min_side| &boundary_cycles.cycles[cycle_min_side])
    }

    pub fn area(&self, boundary_cycles: &BoundaryCycles) -> Vec<Pixel> {
        area_left_of_boundary(self.iter_sides(&boundary_cycles))
    }
}

pub struct ConnectedCycleGroups {
    pub cycle_to_outer_cycle: BTreeMap<CycleMinSide, CycleMinSide>,
    pub outer_cycle_to_group: BTreeMap<CycleMinSide, CycleGroup>,
}

impl ConnectedCycleGroups {
    /// Group together cycles if they are connected by a region of constant material, or in other
    /// words by a path of cells without crossing any boundary sides.
    /// From the minimal side of a cycle we follow the path cells to the top left until we encounter
    /// another cycle. The two cycles belong to the same group.
    /// If a cycle is counter-clockwise it's the outer border of a region.
    /// Returns a map from the min side of a cycle to the outer cycle of the cycle group.
    pub fn from_cycles(cycles: &BoundaryCycles) -> Self {
        // Maps minimal side of a cycle to the minimal side of the outer cycle
        let mut cycle_to_outer_cycle: BTreeMap<CycleMinSide, CycleMinSide> = BTreeMap::new();

        for &min_side in cycles.cycles.keys() {
            // Find out if cycle is ccw or cw, meaning it's an outer or inner border.
            assert!(min_side.name == SideName::Left || min_side.name == SideName::BottomRight);

            if is_outer_border(min_side) {
                cycle_to_outer_cycle.insert(min_side, min_side);
            } else {
                // Move to the opposite side until we reach another cycle
                let mut walk_side = min_side;
                loop {
                    // Opposite side is TopLeft of the same cell
                    walk_side = walk_side.opposite();
                    if let Some(hit_min_side) = cycles.side_to_cycle.get(&walk_side) {
                        let &outer_cycle = cycle_to_outer_cycle.get(hit_min_side).unwrap();
                        cycle_to_outer_cycle.insert(min_side, outer_cycle);
                        break;
                    }
                    // BottomRight side of the top left cell
                    walk_side = walk_side.reversed();
                }
            }
        }

        // TODO: Possible move this to separate function, might not always be required.

        // Maps the minimal side of an outer cycle to the minimal sides of all connected cycles, first
        // item in the list is the minimal side of the outer cycle itself.
        let mut outer_cycle_to_group: BTreeMap<CycleMinSide, CycleGroup> = BTreeMap::new();
        for (&cycle_min_side, &outer_cycle_min_side) in cycle_to_outer_cycle.iter() {
            match outer_cycle_to_group.entry(outer_cycle_min_side) {
                Entry::Vacant(vacant) => {
                    assert_eq!(cycle_min_side, outer_cycle_min_side);
                    let bounds = cycles.cycles[&cycle_min_side].bounds;
                    vacant.insert(CycleGroup::new(cycle_min_side, bounds));
                }
                Entry::Occupied(mut occupied) => occupied.get_mut().add_inner_cycle(cycle_min_side),
            }
        }

        Self {
            cycle_to_outer_cycle,
            outer_cycle_to_group,
        }
    }

    /// Sorted by min_side
    pub fn groups(&self) -> impl Iterator<Item = &CycleGroup> + Clone {
        self.outer_cycle_to_group.values()
    }
}

// fn compute_topology(material_map: &MaterialMap) {
//     let cycle_groups = group_connected_cycles(&cycles, &cycle_map);
//
//     // Create Boundaries from grouped cycles
// }

#[cfg(test)]
mod test {
    use crate::{
        new_regions::{BoundaryCycles, ConnectedCycleGroups},
        pixmap::MaterialMap,
    };

    fn check_repainted_from_cycle_groups(name: &str) {
        let folder = "test_resources/new_regions";
        let path = format!("{folder}/{name}.png");
        let material_map = MaterialMap::load(path).unwrap();

        let boundary_cycles = BoundaryCycles::from_material_map(&material_map);
        let cycle_group = ConnectedCycleGroups::from_cycles(&boundary_cycles);

        // for group in cycle_group.outer_cycle_to_group.values() {
        //     println!("Group:");
        //     for &cycle_min_side in group {
        //         println!("Min side: {cycle_min_side}");
        //     }
        // }

        // Paint found regions back to a material_map and check if it is the same as the original.
        let mut repainted_material_map = MaterialMap::nones_like(&material_map);

        for group in cycle_group.outer_cycle_to_group.values() {
            let material = material_map.get(group.interior_pixel()).unwrap();
            for pixel in group.area(&boundary_cycles) {
                let before = repainted_material_map.set(pixel, material);
                // Make sure areas don't intersect
                assert!(before.is_none());
            }
        }

        // repainted_material_map
        //     .save(format!("{folder}/{name}.repainted.png"))
        //     .unwrap()

        assert_eq!(&material_map, &repainted_material_map);
    }

    #[test]
    fn repaint_from_cycle_groups_a() {
        check_repainted_from_cycle_groups("a");
    }

    #[test]
    fn repaint_from_cycle_groups_b() {
        check_repainted_from_cycle_groups("b");
    }

    #[test]
    fn repaint_from_cycle_groups_c() {
        check_repainted_from_cycle_groups("c");
    }

    #[test]
    fn repaint_from_cycle_groups_d() {
        check_repainted_from_cycle_groups("d");
    }

    #[test]
    fn repaint_from_cycle_groups_e() {
        check_repainted_from_cycle_groups("e");
    }

    #[test]
    fn repaint_from_cycle_groups_f() {
        check_repainted_from_cycle_groups("f");
    }

    #[test]
    fn repaint_from_cycle_groups_g() {
        check_repainted_from_cycle_groups("g");
    }
}
