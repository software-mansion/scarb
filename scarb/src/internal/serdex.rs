use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::internal::fsx;

/// Merge two `toml::Value` serializable structs.
pub fn toml_merge<'de, T, S>(target: &T, source: &S) -> anyhow::Result<T>
where
    T: Serialize + Deserialize<'de>,
    S: Serialize + Deserialize<'de>,
{
    let mut params = toml::Value::try_from(target)?;
    let source = toml::Value::try_from(source)?;

    params.as_table_mut().unwrap().extend(
        source
            .as_table()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone())),
    );
    Ok(toml::Value::try_into(params)?)
}

/// Type representing a path for use in `Scarb.toml` where all paths are expected to be relative to
/// it.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RelativeUtf8PathBuf(Utf8PathBuf);

impl RelativeUtf8PathBuf {
    pub fn relative_to_directory(&self, root: &Utf8Path) -> Result<Utf8PathBuf> {
        fsx::canonicalize_utf8(root.join(&self.0))
    }

    pub fn relative_to_file(&self, file: &Utf8Path) -> Result<Utf8PathBuf> {
        let root = file.parent().expect("Expected file path to not be `/`.");
        self.relative_to_directory(root)
    }
}
