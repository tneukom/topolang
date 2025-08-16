use crate::{
    compiler::GenericRule,
    field::Field,
    interpreter::RuleApplication,
    math::{pixel::Pixel, point::Point, rect::Rect},
    topology::ModificationTime,
    utils::monotonic_time,
};
use ahash::HashMap;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct Glow {
    pub id: u16,
    pub bounds: Rect<i64>,
}

#[derive(Debug, Clone)]
pub struct RuleActivity {
    pub glow_ids: Field<u16>,

    pub glow_alphas: Field<u8>,

    pub glows: HashMap<ModificationTime, Glow>,

    /// (mtime, application_time) pairs
    pub applications: VecDeque<RuleApplication>,
}

impl RuleActivity {
    fn area_glow_alpha(area: impl Iterator<Item = Pixel> + Clone) -> Field<u8> {
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

    pub fn new(bounds: Rect<i64>, rules: &[GenericRule]) -> Self {
        let mut glow_alphas = Field::filled(bounds, 0);
        let mut glow_ids = Field::filled(bounds, 0);
        let mut glows: HashMap<ModificationTime, Glow> = HashMap::default();

        for generic_rule in rules {
            let Some(source) = &generic_rule.source else {
                continue;
            };

            let area = source.area();
            let alphas = Self::area_glow_alpha(area.iter().copied());

            let glow_id = glows.len().try_into().unwrap();
            glows.insert(
                source.modified_time,
                Glow {
                    id: glow_id,
                    bounds: alphas.bounds(),
                },
            );

            for (pixel, &alpha) in alphas.enumerate() {
                if alpha > 0 {
                    glow_alphas.try_set(pixel, alpha).ok();
                    glow_ids.try_set(pixel, glow_id).ok();
                }
            }
        }

        // For debugging
        // let mut alphas_rgba = glow_alphas.map(|&alpha| Rgba(alpha, 0, 0, 255));
        // alphas_rgba.save("wtf.png");

        Self {
            glow_ids,
            glow_alphas,
            glows,
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

    pub fn glow_intensities(&self) -> HashMap<ModificationTime, f64> {
        let real_time = monotonic_time();

        let mut intensities = HashMap::default();
        for application in &self.applications {
            let Some(rule_mtime) = application.source_mtime else {
                continue;
            };

            // intensity is 1 if real_time == application.real_time and 0 delta_t later, so
            let animation_time = real_time - application.real_time;
            if animation_time > 0.0 {
                let intensity = 1.0 - 3.0 * animation_time;
                if intensity > 0.0 {
                    intensities.insert(rule_mtime, intensity);
                }
            }
        }
        intensities
    }
}
