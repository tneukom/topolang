use crate::{
    math::{pixel::Pixel, rgba8::Rgba8},
    pattern::{find_matches, NullTrace},
    pixmap::Pixmap,
    rule::Rule,
    topology::{BorderKey, Topology},
};

pub struct Compiler {
    rule_frame: Topology,
    before_border: BorderKey,
    after_border: BorderKey,
}

impl Compiler {
    const RULE_FRAME_VOID_COLOR: Rgba8 = Rgba8::CYAN;

    pub fn new() -> anyhow::Result<Self> {
        // Load rule_frame pattern from file
        let rule_frame_pixmap = Pixmap::from_bitmap_path("resources/rule_frame.png")?
            .without_color(Self::RULE_FRAME_VOID_COLOR);
        let rule_frame = Topology::new(&rule_frame_pixmap);

        // Side on the before border (inner border of the frame)
        let before_side = Pixel::new(7, 7).top_side().reversed();
        let Some((before_border, _)) = rule_frame.border_containing_side(&before_side) else {
            anyhow::bail!("Before border not found.")
        };

        let after_side = Pixel::new(43, 8).top_side().reversed();
        let Some((after_border, _)) = rule_frame.border_containing_side(&after_side) else {
            anyhow::bail!("After border not found.")
        };

        Ok(Self {
            rule_frame,
            before_border,
            after_border,
        })
    }

    pub fn compile(&self, world: &mut Topology) -> anyhow::Result<Vec<Rule>> {
        // Find all matches for rule_frame in world
        let matches = find_matches(world, &self.rule_frame, NullTrace::new());

        // Extract all matches and creates rules from them
        let mut world_pixmap = world.to_pixmap();
        let mut rules: Vec<Rule> = Vec::new();
        for phi in matches {
            // Extract before and after from rule
            let phi_before_border = phi[self.before_border];
            let before_pixmap =
                world_pixmap.extract_right(&world[phi_before_border], Some(Rgba8::TRANSPARENT));
            let phi_after_border = phi[self.after_border];
            let after_pixmap =
                world_pixmap.extract_right(&world[phi_after_border], Some(Rgba8::TRANSPARENT));

            // Find translation from after to before
            let before_bounds = before_pixmap.bounds();
            let after_bounds = after_pixmap.bounds();
            assert_eq!(before_bounds.size(), after_bounds.size());

            let offset = before_bounds.low() - after_bounds.low();
            let after_pixmap = after_pixmap.translated(offset);

            let rule = Rule::new(&before_pixmap, &after_pixmap)?;
            rules.push(rule);

            // Clear the frame of the compiled rule from the world
            for &region_key in self.rule_frame.iter_region_keys() {
                let phi_region = &world[phi[region_key]];
                for &pixel in &phi_region.interior {
                    world_pixmap.set(pixel, Rgba8::TRANSPARENT);
                }
            }
        }

        *world = Topology::new(&world_pixmap);
        Ok(rules)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        bitmap::Bitmap, compiler::Compiler, pixmap::Pixmap, rule::stabilize, topology::Topology,
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn init() {
        let _compiler = Compiler::new().unwrap();
    }

    fn assert_execute_world(name: &str, expected_steps: usize) {
        let folder = format!("test_resources/compiler/{name}/");
        let world_bitmap = Bitmap::from_path(format!("{folder}/world.png")).unwrap();
        let world_pixmap = Pixmap::from_bitmap(&world_bitmap);
        let mut world = Topology::new(&world_pixmap);

        let compiler = Compiler::new().unwrap();
        let rules = compiler.compile(&mut world).unwrap();

        let steps = stabilize(&mut world, &rules);
        assert_eq!(steps, expected_steps);

        let result_pixmap = world.to_pixmap();

        let expected_pixmap =
            Pixmap::from_bitmap_path(format!("{folder}/world_expected.png")).unwrap();
        assert_eq!(result_pixmap, expected_pixmap);

        // let result_bitmap = result_pixmap.to_bitmap_with_size(world_bitmap.size());
        // result_bitmap
        //     .save(format!("{folder}/world_result.png"))
        //     .unwrap();
    }

    #[test]
    fn a() {
        assert_execute_world("a", 4);
    }

    #[test]
    fn b() {
        assert_execute_world("b", 7);
    }

    #[test]
    fn c() {
        assert_execute_world("c", 2);
    }

    #[test]
    fn gate() {
        assert_execute_world("gate", 3);
    }
}
