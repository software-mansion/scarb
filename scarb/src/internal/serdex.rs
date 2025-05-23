use anyhow::{Result, anyhow};
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::internal::fsx;

const MERGE_STRATEGY_KEY: &str = "merge-strategy";

#[derive(Debug, Default)]
pub enum MergeStrategy {
    #[default]
    Override,
    Merge,
}

impl TryFrom<&str> for MergeStrategy {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "override" => Ok(Self::Override),
            "merge" => Ok(Self::Merge),
            _ => Err(anyhow!(
                "invalid merge strategy: {}, must be one of `merge`, `override`",
                value
            )),
        }
    }
}

/// Merge `source` into `target` where `source` and `target` are two `toml::Value` serializable
/// structs.
/// If `source` and `target` are tables and a specific key exists in both, the conflict will be
/// resolved as follows:
/// 1) If value under the conflicting key in either `source` or `target` is not a table, the value
///    in `target` will be overridden with the value from `source`. This means that no keys from
///    subtables under the conflicting key will be preserved.
/// 2) If values under the conflicting key in both `source` and `target` are tables,
///    the conflict will be resolved with one of two strategies defined by `source`:
///    a) If `source` have a key `merge-strategy` with value `override`, the value in `target` will
///    be overridden with the value from `source`.
///    b) If `source` have a key `merge-strategy` with value `merge`, the value in `target` will be
///    merged with the value from `source` recursively with this function.
///    c) If `source` does not have a key `merge-strategy`, the value in `target` will be overridden.
pub fn toml_merge_apply_strategy<'de, T, S>(target: &T, source: &S) -> Result<T>
where
    T: Serialize + Deserialize<'de>,
    S: Serialize + Deserialize<'de>,
{
    let mut target_value = toml::Value::try_from(target)?;
    let source_value = toml::Value::try_from(source)?;

    if let (Some(target_table), Some(source_table)) =
        (target_value.as_table_mut(), source_value.as_table())
    {
        for (key, value) in source_table {
            match target_table.get_mut(key) {
                Some(target_subvalue) if target_subvalue.is_table() && value.is_table() => {
                    let target_subtable = target_subvalue.as_table_mut().unwrap();
                    let value_subtable = value.as_table().unwrap();
                    let strategy = value_subtable
                        .get(MERGE_STRATEGY_KEY)
                        .and_then(|v| v.as_str())
                        .map_or(Ok(MergeStrategy::default()), MergeStrategy::try_from)?;
                    match &strategy {
                        MergeStrategy::Override => {
                            *target_subvalue = toml::Value::try_from(value_subtable.clone())?;
                        }
                        MergeStrategy::Merge => {
                            *target_subvalue = toml::Value::try_from(toml_merge_apply_strategy(
                                target_subtable,
                                value_subtable,
                            )?)?;
                        }
                    }
                }
                _ => {
                    target_table.insert(key.clone(), value.clone());
                }
            }
        }
    }

    Ok(toml::Value::try_into(target_value)?)
}

/// Merge `source` into `target` where `source` and `target` are two `toml::Value` serializable
/// structs.
/// If `source` and `target` are tables and a specific key exists in both, the value in `target`
/// will be overridden with the value from `source`.
pub fn toml_merge<'de, T, S>(target: &T, source: &S) -> Result<T>
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

#[cfg(test)]
mod tests {
    use super::{toml_merge, toml_merge_apply_strategy};
    use test_case::test_case;

    #[test_case(r#"{}"#, r#"{}"#)]
    #[test_case(r#"{"a": "a"}"#, r#"{"b":"b"}"#)]
    #[test_case(r#"{"a": "a"}"#, r#"{"a":"b"}"#)]
    #[test_case(r#"{"a": {"a": "a"}}"#, r#"{"a": {"a": "b"}}"#)]
    #[test_case(
        r#"{"a": {"a": "a", "merge-strategy": "merge"}}"#,
        r#"{"a": {"a": "b"}}"#
    )]
    #[test_case(
        r#"{"a": {"a": "a", "merge-strategy": "override"}}"#,
        r#"{"a": {"a": "b", "b": "b"}}"#
    )]
    fn merge_with_override(source: &'static str, target: &'static str) {
        let source: toml::Value = serde_json::from_str(source).unwrap();
        let target: toml::Value = serde_json::from_str(target).unwrap();
        assert_eq!(
            toml_merge(&target, &source).unwrap(),
            toml_merge_apply_strategy(&target, &source).unwrap()
        );
    }

    #[test]
    fn merge_with_merge_strategy() {
        let target = r#"{"a": {"a": "b", "b": "b"}}"#;
        let source = r#"{"a": {"a": "a", "merge-strategy": "merge"}}"#;
        let target: toml::Value = serde_json::from_str(target).unwrap();
        let source: toml::Value = serde_json::from_str(source).unwrap();
        let with_override = toml_merge(&target, &source).unwrap();
        let with_merge = toml_merge_apply_strategy(&target, &source).unwrap();
        let with_override = serde_json::to_string(&with_override).unwrap();
        let with_merge = serde_json::to_string(&with_merge).unwrap();
        assert_eq!(with_override, r#"{"a":{"a":"a","merge-strategy":"merge"}}"#);
        assert_eq!(
            with_merge,
            r#"{"a":{"a":"a","b":"b","merge-strategy":"merge"}}"#
        );
    }

    #[test_case(
        r#"{"a": {"a": "b", "b": "b"}}"#,
        r#"{"a": {"a": "a"}, "merge-strategy": "merge"}"#
    )]
    #[test_case(
        r#"{"a": {"merge-strategy": "merge", "a": "b", "b": "b"}}"#,
        r#"{"a": {"a": "a"}}"#
    )]
    fn merge_strategy_must_be_on_source(target: &'static str, source: &'static str) {
        let source: toml::Value = serde_json::from_str(source).unwrap();
        let target: toml::Value = serde_json::from_str(target).unwrap();
        assert_eq!(
            toml_merge(&target, &source).unwrap(),
            toml_merge_apply_strategy(&target, &source).unwrap()
        );
    }

    #[test]
    fn invalid_merge_strategy() {
        let target = r#"{"a": {"a": "b", "b": "b"}}"#;
        let source = r#"{"a": {"a": "a", "merge-strategy": "other"}}"#;
        let target: toml::Value = serde_json::from_str(target).unwrap();
        let source: toml::Value = serde_json::from_str(source).unwrap();
        assert!(toml_merge_apply_strategy(&target, &source).is_err());
    }
}
