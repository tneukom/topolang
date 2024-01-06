use crate::{
    math::{
        direction::Direction,
        pixel::{Side, Vertex},
        rgba8::Rgba8,
    },
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

        // Side on the before border
        let before_side = Side::new(Vertex::new(7, 7), Direction::Right);
        let Some((before_border, _)) = rule_frame.border_containing_side(&before_side) else {
            anyhow::bail!("Before border not found.")
        };

        let after_side = Side::new(Vertex::new(43, 8), Direction::Right);
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
            let before_pixmap = world_pixmap.extract_right(&world[phi_before_border]);
            let phi_after_border = phi[self.after_border];
            let after_pixmap = world_pixmap.extract_right(&world[phi_after_border]);

            // Find translation from after to before
            let before_bounds = before_pixmap.bounds();
            let after_bounds = after_pixmap.bounds();
            assert_eq!(before_bounds.size(), after_bounds.size());

            let offset = before_bounds.low() - after_bounds.low();
            let after_pixmap = after_pixmap.translated(offset);

            let rule = Rule::new(&before_pixmap, &after_pixmap)?;
            rules.push(rule);

            // Clear rule frame
            for &region_key in self.rule_frame.iter_region_keys() {
                let phi_region = &world[phi[region_key]];
                world_pixmap.fill(Rgba8::TRANSPARENT, &phi_region.interior);
            }
        }

        *world = Topology::new(&world_pixmap);
        Ok(rules)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        bitmap::Bitmap,
        compiler::Compiler,
        math::rgba8::Rgba8,
        pattern::{find_first_match, NullTrace},
        pixmap::Pixmap,
        rule::stabilize,
        topology::Topology,
    };

    #[test]
    fn init() {
        let _compiler = Compiler::new().unwrap();
    }

    #[test]
    fn a() {
        let folder = "test_resources/compiler/gate/";
        let world_bitmap = Bitmap::from_path(format!("{folder}/world.png")).unwrap();
        let world_pixmap = Pixmap::from_bitmap(&world_bitmap);
        let mut world = Topology::new(&world_pixmap);

        let compiler = Compiler::new().unwrap();
        let rules = compiler.compile(&mut world).unwrap();
        // assert_eq!(rules.len(), 2);

        let application_count = stabilize(&mut world, &rules);
        // assert_eq!(application_count, 3);

        let result_pixmap = world.to_pixmap();
        let result_bitmap = result_pixmap.to_bitmap_with_size(world_bitmap.size());
        result_bitmap
            .save(format!("{folder}/world_result.png"))
            .unwrap();
    }
}
