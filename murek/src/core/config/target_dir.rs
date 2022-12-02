use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::Result;

use create_output_dir::create_output_dir;

use crate::internal::fsx;

pub struct TargetDir {
    // This is deliberately private to enforce usage of profiles.
    pub path: PathBuf,
}

pub struct ProfileTargetDir {
    pub path: PathBuf,
}

impl TargetDir {
    #[tracing::instrument(name = "target_dir_init", level = "trace")]
    pub fn init(workspace_root: &Path) -> Result<Self> {
        let path = workspace_root.join("target");

        // TODO(mkaput): Call this lazily once in this object's lifetime.
        create_output_dir(&path)?;

        Ok(Self { path })
    }

    #[tracing::instrument(name = "target_dir_profile_init", level = "trace", skip(self))]
    pub fn profile(&self, name: &str) -> Result<ProfileTargetDir> {
        let path = self.path.join(name);

        // TODO(mkaput): Call this lazily once in this object's lifetime for each profile.
        fsx::create_dir_all(&path)?;

        Ok(ProfileTargetDir { path })
    }

    pub(crate) fn clean(&self) -> Result<()> {
        fsx::remove_dir_all(&self.path)
    }
}

impl fmt::Debug for TargetDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TargetDir")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}
