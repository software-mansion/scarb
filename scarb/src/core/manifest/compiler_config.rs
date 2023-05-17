use serde::{Deserialize, Serialize};

use crate::compiler::{DefaultForProfile, Profile};
use crate::core::TomlCairo;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct ManifestCompilerConfig {
    /// Replace all names in generated Sierra code with dummy counterparts, representing the
    /// expanded information about the named items.
    ///
    /// For libfuncs and types that would be recursively opening their generic arguments.
    /// For functions, that would be their original name in Cairo.
    /// For example, while the Sierra name be `[6]`, with this flag turned on it might be:
    /// - For libfuncs: `felt252_const<2>` or `unbox<Box<Box<felt252>>>`.
    /// - For types: `felt252` or `Box<Box<felt252>>`.
    /// - For user functions: `test::foo`.
    pub sierra_replace_ids: bool,
}

impl DefaultForProfile for ManifestCompilerConfig {
    fn default_for_profile(profile: &Profile) -> Self {
        Self {
            sierra_replace_ids: profile.is_dev(),
        }
    }
}

impl From<ManifestCompilerConfig> for TomlCairo {
    fn from(config: ManifestCompilerConfig) -> Self {
        Self {
            sierra_replace_ids: Some(config.sierra_replace_ids),
        }
    }
}
