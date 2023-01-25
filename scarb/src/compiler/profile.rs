use smol_str::SmolStr;

use crate::core::Config;
#[cfg(doc)]
use crate::core::Target;
use crate::flock::Filesystem;

/// Profile settings used to determine which compiler flags to use for a [`Target`].
#[derive(Clone, Debug)]
pub struct Profile {
    pub name: SmolStr,
}

impl Profile {
    pub fn target_dir<'c>(&self, config: &'c Config) -> Filesystem<'c> {
        config.target_dir().child(self.name.as_str())
    }
}
