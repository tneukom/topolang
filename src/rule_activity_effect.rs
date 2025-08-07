use crate::{
    compiler::Program, interpreter::RuleApplication, math::pixel::Side, topology::ModificationTime,
    utils::monotonic_time,
};
use ahash::HashMap;
use std::{collections::VecDeque, sync::Arc};

pub type Outlines = HashMap<ModificationTime, Vec<Side>>;

#[derive(Debug, Clone, Default)]
pub struct RuleActivity {
    pub rule_outlines: Arc<Outlines>,

    /// (mtime, application_time) pairs
    pub applications: VecDeque<RuleApplication>,
}

impl RuleActivity {
    pub fn update_program(&mut self, program: &Program) {
        let rule_outlines = program
            .rules
            .iter()
            .filter_map(|rule| {
                rule.source.as_ref().map(|source| {
                    let sides: Vec<_> = source.outline().iter().copied().collect();
                    (source.modified_time, sides)
                })
            })
            .collect();
        self.rule_outlines = Arc::new(rule_outlines);
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
}
