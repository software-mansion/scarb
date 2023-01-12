use std::collections::HashMap;

use id_arena::Arena;
use tracing::trace;

use crate::resolver::incompatibility::{Incompatibility, IncompatibilityId};
use crate::resolver::package_ref::PackageRef;

const TYPICAL_INCOMPATIBILITIES_COUNT: usize = 128;
const TYPICAL_PACKAGES_COUNT: usize = 8;

pub struct IncompatibilitySet {
    arena: Arena<Incompatibility>,
    incompatibilities: HashMap<PackageRef, Vec<IncompatibilityId>>,
}

impl IncompatibilitySet {
    pub fn new() -> Self {
        Self {
            arena: Arena::with_capacity(TYPICAL_INCOMPATIBILITIES_COUNT),
            incompatibilities: HashMap::with_capacity(TYPICAL_PACKAGES_COUNT),
        }
    }

    pub fn insert(&mut self, incompatibility: Incompatibility) -> IncompatibilityId {
        trace!("fact {incompatibility}");
        let id = self.arena.alloc(incompatibility);
        self.merge_incompatibility(id);
        id
    }

    fn merge_incompatibility(&mut self, id: IncompatibilityId) {
        for term in self.arena[id].terms() {
            self.incompatibilities
                .entry(term.package_range.name.clone())
                .or_default()
                .push(id);
        }
    }
}
