use crate::{
    material::Material,
    math::pixel::Side,
    morphism::Morphism,
    pattern::{NullTrace, SearchMorphism},
    topology::{Boundary, Region, Seam, Topology},
    world::World,
};
use itertools::Itertools;

pub struct FillBoundary {
    pub boundary: Vec<Side>,
    pub material: Material,
}

pub struct Rule {
    /// The pattern
    pub before: Topology,

    /// The substitution
    pub after: Topology,
}

impl Rule {
    #[inline(never)]
    pub fn new(before: Topology, after: Topology) -> anyhow::Result<Self> {
        // Make sure after region map is constant on each region in before.
        for (&region_key, region) in before.regions.iter() {
            // REVISIT: Could be faster by iterating over both before.region_map and
            //   after.region_map in parallel.
            let all_equal = before
                .iter_region_interior(region_key)
                .map(|pixel| after.material_map[pixel])
                .all_equal();
            if !all_equal {
                anyhow::bail!("Invalid rule, substitution region not constant.")
            }
        }

        Ok(Rule { before, after })
    }

    /// Extend the seam mapping of `phi` to reversed seams.
    pub fn extended_morphism_seam_map(
        codom: &Topology,
        phi: &Morphism,
        seam: Seam,
    ) -> Option<Vec<Seam>> {
        // TODO: Could return Option<Iterator> instead!
        // Try to map seam directly
        if let Some(&phi_seam) = phi.seam_map.get(&seam) {
            return Some(codom.seam_atoms(phi_seam).unwrap().collect());
        }

        // Try to map seam.reversed() instead, and then reverse it back
        if let Some(&phi_reversed_seam) = phi.seam_map.get(&seam.atom_reversed()) {
            // phi_reversed_seam is possible not an atom, so we cannot simply reverse it back.
            let phi_seam = codom.reverse_seam(phi_reversed_seam).unwrap().collect();
            return Some(phi_seam);
        }

        None
    }

    /// Map the boundary a region `region` in the domain of phi to its boundary in the co-domain.
    /// This works if for every `seam` in the boundary of region, `phi_hat(seam)` is defined.
    /// `phi_hat` is the unique extension of `phi` that for each `seam` also maps `seam.reversed()`.
    /// The result is a set of seams in `codom` but not necessarily the boundary of a Region, we
    /// return directly the
    pub fn map_region_boundary(
        &self,
        codom: &Topology,
        phi: &Morphism,
        boundary: &Boundary,
    ) -> Vec<Seam> {
        let mut phi_boundary: Vec<Seam> = Vec::new();

        // Maps seams into world to construct new boundary.
        for seam in boundary.iter_seams() {
            let phi_seam = Self::extended_morphism_seam_map(codom, phi, seam).unwrap();
            phi_boundary.extend(phi_seam)
            // let sides = Self::extended_topology_iter_seam_sides(codom, phi_seam).unwrap();
            // boundary.extend(sides);
        }

        phi_boundary
    }

    /// Compute the fill operation for the before -> after substitution for the given
    /// `before_region`.
    /// To fill a region with a material we need to map its boundary into `world` using the Morphism
    /// phi. The result is in general not a Seam, but a SeamPath. We directly return the sides
    /// in the SeamPath.
    pub fn region_substitution(
        &self,
        world: &Topology,
        phi: &Morphism,
        before_region: &Region,
    ) -> Option<FillBoundary> {
        let after_material = self.after.material_map[before_region.arbitrary_interior_pixel()];
        if before_region.material == after_material {
            // No painting necessary
            return None;
        }

        // A seam path
        let phi_boundary = self.map_region_boundary(world, phi, &before_region.boundary);

        let phi_boundary_sides = phi_boundary
            .iter()
            .flat_map(|&seam| {
                assert!(seam.is_atom());
                println!("seam: {seam}");
                world.iter_seam_sides(seam).unwrap()
            })
            .collect();

        let fill = FillBoundary {
            boundary: phi_boundary_sides,
            material: after_material,
        };

        Some(fill)
    }

    /// Given a match for the pattern `self.before` and the world, apply the substitution determined
    /// by `self.before` and `self.after`
    /// Returns true if there were any changes to the world
    pub fn substitute(&self, phi: &Morphism, world: &mut World) -> bool {
        let mut fills = Vec::new();

        for before_region in self.before.regions.values() {
            if let Some(fill) = self.region_substitution(world.topology(), phi, before_region) {
                fills.push(fill);
            }
        }

        for fill in &fills {
            world.fill_boundary(fill);
        }

        fills.len() > 0
    }
}

pub fn stabilize(world: &mut World, rules: &Vec<Rule>) -> usize {
    let mut steps: usize = 0;
    loop {
        let mut applied = false;
        for rule in rules {
            if let Some(phi) = SearchMorphism::new(world.topology(), &rule.before)
                .find_first_match(NullTrace::new())
            {
                rule.substitute(&phi, world);
                steps += 1;
                applied = true;
            }
        }

        if !applied {
            return steps;
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        field::RgbaField,
        material::Material,
        pattern::{NullTrace, SearchMorphism},
        pixmap::MaterialMap,
        rule::Rule,
        topology::Topology,
        utils::IntoT,
        world::World,
    };

    fn assert_rule_application(folder: &str, expected_application_count: usize) {
        let folder = format!("test_resources/rules/{folder}");

        let before_material_map = RgbaField::load(format!("{folder}/before.png"))
            .unwrap()
            .intot::<MaterialMap>()
            .without(&Material::VOID);
        let before = Topology::new(before_material_map);

        let after_material_map = RgbaField::load(format!("{folder}/after.png"))
            .unwrap()
            .intot::<MaterialMap>()
            .without(&Material::VOID);
        let after = Topology::new(after_material_map);

        let rule = Rule::new(before, after).unwrap();

        let world_material_map = RgbaField::load(format!("{folder}/world.png"))
            .unwrap()
            .into();
        let mut world = World::from_material_map(world_material_map);

        let mut application_count: usize = 0;
        while let Some(phi) =
            SearchMorphism::new(world.topology(), &rule.before).find_first_match(NullTrace::new())
        {
            rule.substitute(&phi, &mut world);

            // Save world to image for debugging!
            // world
            //     .topology()
            //     .material_map
            //     .to_field(Material::BLACK)
            //     .into_rgba()
            //     .save(format!("{folder}/run_{application_count}.png"))
            //     .unwrap();

            application_count += 1;
            assert!(application_count <= expected_application_count);
        }

        assert_eq!(application_count, expected_application_count);

        let result_pixmap = world.material_map();
        let expected_result_pixmap = RgbaField::load(format!("{folder}/expected_result.png"))
            .unwrap()
            .into();

        // result_pixmap
        //     .to_field(Material::TRANSPARENT)
        //     .into_rgba()
        //     .save(format!("{folder}/world_result.png"))
        //     .unwrap();

        assert_eq!(result_pixmap, &expected_result_pixmap);
    }

    // #[test]
    // fn hunt_bug() {
    //     assert_rule_application("hunt_bug", 1)
    // }

    #[test]
    fn rule_a() {
        assert_rule_application("a", 3)
    }

    #[test]
    fn rule_b() {
        assert_rule_application("b", 3)
    }

    #[test]
    fn rule_c() {
        assert_rule_application("c", 3)
    }

    #[test]
    fn rule_circles_1() {
        assert_rule_application("circles_1", 2)
    }

    #[test]
    fn rule_circles_2() {
        assert_rule_application("circles_2", 3)
    }

    #[test]
    fn rule_gate_a() {
        assert_rule_application("gate_a", 2)
    }

    #[test]
    fn rule_interior_hole() {
        assert_rule_application("interior_hole", 1)
    }

    #[test]
    fn rule_two_regions_failure_case() {
        assert_rule_application("two_regions_failure_case", 1)
    }
}
