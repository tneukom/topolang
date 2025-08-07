use crate::{
    math::affine_map::AffineMap,
    painting::wave_painter::{WaveGeometry, WavePainter},
    rule_activity_effect::{Outlines, RuleActivity},
    topology::ModificationTime,
};
use ahash::HashMap;
use std::sync::Arc;

pub struct RuleActivityPainter {
    pub wave_painter: WavePainter,
    pub wave_geometries: HashMap<ModificationTime, WaveGeometry>,
    pub outlines: Arc<Outlines>,
}

impl RuleActivityPainter {
    pub unsafe fn new(gl: &glow::Context) -> Self {
        Self {
            wave_painter: WavePainter::new(gl),
            wave_geometries: Default::default(),
            outlines: Default::default(),
        }
    }

    pub unsafe fn draw(
        &mut self,
        gl: &glow::Context,
        rule_activity: &RuleActivity,
        view_from: AffineMap<f64>,
        device_from_view: AffineMap<f64>,
        time: f64,
    ) {
        if !Arc::ptr_eq(&self.outlines, &rule_activity.rule_outlines) {
            self.wave_geometries = rule_activity
                .rule_outlines
                .iter()
                .map(|(&mtime, boundary)| {
                    let geometry = WaveGeometry::from_border(boundary);
                    (mtime, geometry)
                })
                .collect();
            self.outlines = rule_activity.rule_outlines.clone();
        }

        for rule_application in &rule_activity.applications {
            println!("rule_application: {}", rule_application.real_time);

            let Some(source_mtime) = rule_application.source_mtime else {
                continue;
            };

            let wave_radius = 10.0 * (time - rule_application.real_time);
            if wave_radius > 10.0 {
                continue;
            }

            let geometry = self.wave_geometries.get(&source_mtime).unwrap();
            self.wave_painter
                .draw_wave(gl, geometry, view_from, device_from_view, wave_radius);
        }
    }
}
