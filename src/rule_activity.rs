use crate::{
    compiler::GenericRule,
    field::Field,
    interpreter::RuleApplication,
    math::{pixel::Pixel, point::Point, rect::Rect},
    painting::glow_painter::Glow,
    topology::ModificationTime,
    utils::monotonic_time,
};
use ahash::HashMap;
use std::{collections::VecDeque, sync::Arc};

#[derive(Debug, Clone)]
pub struct RuleActivity {
    /// Maps rule modification time to outline bitmap
    pub outlines: HashMap<ModificationTime, Arc<Field<u8>>>,

    /// (mtime, application_time) pairs
    pub applications: VecDeque<RuleApplication>,
}

impl RuleActivity {
    fn area_outline(area: impl Iterator<Item = Pixel> + Clone) -> Field<u8> {
        let bounds = Rect::index_bounds(area.clone()).padded(2);
        let mut alpha = Field::filled(bounds, 0);

        // Set all pixels + kernel
        for pixel in area.clone() {
            for offset_y in -1..=1 {
                for offset_x in -1..=1 {
                    let offset_pixel = pixel + Point(offset_x, offset_y);
                    alpha.set(offset_pixel, 255);
                }
            }
        }

        // Remove all pixels in area
        for pixel in area {
            alpha.set(pixel, 0);
        }

        alpha
    }

    pub fn new(rules: &[GenericRule]) -> Self {
        let mut outlines = HashMap::default();

        for generic_rule in rules {
            let Some(source) = &generic_rule.source else {
                continue;
            };

            let area = source.area();
            let outline = Self::area_outline(area.iter().copied());
            outlines.insert(source.modified_time, Arc::new(outline));
        }

        // For debugging
        // let mut alphas_rgba = glow_alphas.map(|&alpha| Rgba(alpha, 0, 0, 255));
        // alphas_rgba.save("wtf.png");

        Self {
            outlines,
            applications: VecDeque::default(),
        }
    }

    fn discard_obsolete_applications(&mut self) {
        let real_time = monotonic_time();
        let mut pop_obsolete = || {
            let first = self.applications.front_mut()?;
            if first.real_time < real_time - 2.0 {
                self.applications.pop_front()
            } else {
                None
            }
        };

        while let Some(_) = pop_obsolete() {}
    }

    pub fn rule_applied(&mut self, application: RuleApplication) {
        self.applications.push_back(application);
        self.discard_obsolete_applications();
    }

    pub fn glows(&self) -> Vec<Glow> {
        let real_time = monotonic_time();
        let mut glows = Vec::new();

        for application in &self.applications {
            let Some(rule_mtime) = application.source_mtime else {
                continue;
            };

            // intensity is 1 if real_time == application.real_time and 0 delta_t later, so
            let animation_time = real_time - application.real_time;
            if animation_time > 0.0 {
                let alpha = 1.0 - 3.0 * animation_time;
                if alpha > 0.0 {
                    let glow = Glow {
                        outline: self.outlines.get(&rule_mtime).unwrap().clone(),
                        alpha,
                    };
                    glows.push(glow);
                }
            }
        }

        glows
    }
}
