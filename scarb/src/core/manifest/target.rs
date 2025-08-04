use crate::core::{TargetKind, TomlExternalTargetParams};
use crate::internal::restricted_names;
use crate::internal::serdex::toml_merge;
use anyhow::{Result, bail};
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

/// See [`TargetInner`] for public fields reference.
#[derive(Clone, Debug, Hash)]
pub struct Target(Arc<TargetInner>);

#[derive(Debug)]
#[non_exhaustive]
pub struct TargetInner {
    pub kind: TargetKind,
    pub name: SmolStr,
    pub source_path: Utf8PathBuf,
    pub group_id: Option<SmolStr>,
    pub params: toml::Value,
}

impl Deref for Target {
    type Target = TargetInner;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl Target {
    pub fn new(
        kind: TargetKind,
        name: impl Into<SmolStr>,
        source_path: impl Into<Utf8PathBuf>,
        group_id: Option<SmolStr>,
        params: toml::Value,
    ) -> Self {
        assert!(params.is_table(), "params must be a TOML table");
        Self(Arc::new(TargetInner {
            kind,
            name: name.into(),
            source_path: source_path.into(),
            group_id,
            params,
        }))
    }

    pub fn without_params(
        kind: TargetKind,
        name: impl Into<SmolStr>,
        source_path: impl Into<Utf8PathBuf>,
    ) -> Self {
        Self::new(
            kind,
            name,
            source_path,
            None,
            toml::Value::Table(toml::Table::new()),
        )
    }

    pub fn try_from_structured_params(
        kind: TargetKind,
        name: impl Into<SmolStr>,
        source_path: impl Into<Utf8PathBuf> + Clone,
        group_id: Option<SmolStr>,
        params: impl Serialize,
    ) -> Result<Self> {
        Self::validate_test_target_file_stem(source_path.clone().into())?;
        let params = toml::Value::try_from(params)?;
        Ok(Self::new(kind, name, source_path, group_id, params))
    }

    pub fn validate_test_target_file_stem(source_path: Utf8PathBuf) -> Result<()> {
        let file_stem = source_path.file_stem().expect("failed to get file stem");

        if file_stem == ".cairo" {
            bail!(
                "empty string cannot be used as a test target name \
                consider renaming file: {}",
                source_path
            );
        }

        if file_stem == "_" {
            bail!(
                "underscore cannot be used as a test target name \
                consider renaming file: {}",
                source_path
            );
        }

        let mut chars = file_stem.chars();
        if let Some(ch) = chars.next() {
            if ch.is_ascii_digit() {
                bail!(
                    "the name `{file_stem}` cannot be used as a test target, \
                    names cannot start with a digit \
                    consider renaming file: {}",
                    source_path
                );
            }

            if !(ch.is_ascii_alphabetic() || ch == '_') {
                bail!(
                    "invalid character `{ch}` in test target name: `{file_stem}`, \
                    the first character must be an ASCII lowercase letter or underscore \
                    consider renaming file: {}",
                    source_path
                )
            }
        }
        for ch in chars {
            if !(ch.is_ascii_alphanumeric() || ch == '_') {
                bail!(
                    "invalid character `{ch}` in test target name: `{file_stem}`, \
                    characters must be ASCII letters, ASCII numbers or underscore \
                    consider renaming file: {}",
                    source_path
                )
            }
        }

        if restricted_names::is_keyword(file_stem) {
            bail!(
                "the name `{file_stem}` cannot be used as a test target name, \
                names cannot use Cairo keywords see the full list at https://starknet.io/cairo-book/appendix-01-keywords.html \
                consider renaming file: {}",
                source_path
            )
        }

        Ok(())
    }

    pub fn is_lib(&self) -> bool {
        self.kind == TargetKind::LIB
    }

    pub fn is_cairo_plugin(&self) -> bool {
        self.kind == TargetKind::CAIRO_PLUGIN
    }

    pub fn is_test(&self) -> bool {
        self.kind == TargetKind::TEST
    }

    pub fn source_root(&self) -> &Utf8Path {
        self.source_path
            .parent()
            .expect("Source path is guaranteed to point to a file.")
    }

    pub fn props<'de, P>(&self) -> Result<P>
    where
        P: Default + Serialize + Deserialize<'de>,
    {
        toml_merge(&P::default(), &self.params)
    }
}

impl Hash for TargetInner {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.name.hash(state);
        self.source_path.hash(state);
        self.params.to_string().hash(state);
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TestTargetProps {
    pub test_type: TestTargetType,
    pub build_external_contracts: Option<Vec<String>>,
}

impl TestTargetProps {
    pub fn new(test_type: TestTargetType) -> Self {
        Self {
            test_type,
            build_external_contracts: Default::default(),
        }
    }

    pub fn with_build_external_contracts(self, external: Vec<String>) -> Self {
        Self {
            build_external_contracts: Some(external),
            ..self
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TestTargetType {
    #[default]
    Unit,
    Integration,
}

impl TryInto<TomlExternalTargetParams> for TestTargetProps {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<TomlExternalTargetParams, Self::Error> {
        Ok(toml::Value::try_into(toml::Value::try_from(self)?)?)
    }
}

#[cfg(test)]
mod tests {
    use super::Target;
    use camino::Utf8PathBuf;
    use test_case::test_case;

    #[test_case("foo")]
    #[test_case("_bar")]
    fn validate_correct_test_target_name(name: &str) {
        assert!(Target::validate_test_target_file_stem(name.into()).is_ok())
    }

    #[test_case("" => "empty string cannot be used as a test target name consider renaming file: parent/.cairo"; "empty string")]
    #[test_case("_" => "underscore cannot be used as a test target name consider renaming file: parent/_.cairo"; "underscore")]
    #[test_case("1" => "the name `1` cannot be used as a test target, names cannot start with a digit consider renaming file: parent/1.cairo"; "digit")]
    #[test_case("123" => "the name `123` cannot be used as a test target, names cannot start with a digit consider renaming file: parent/123.cairo"; "digits")]
    #[test_case("0foo" => "the name `0foo` cannot be used as a test target, names cannot start with a digit consider renaming file: parent/0foo.cairo"; "digit_and_alphabetic")]
    #[test_case("*abc" => "invalid character `*` in test target name: `*abc`, the first character must be an ASCII lowercase letter or underscore consider renaming file: parent/*abc.cairo"; "not_alphanumeric_start")]
    #[test_case("fo-o" => "invalid character `-` in test target name: `fo-o`, characters must be ASCII letters, ASCII numbers or underscore consider renaming file: parent/fo-o.cairo"; "invalid_character")]
    #[test_case("hint" => "the name `hint` cannot be used as a test target name, names cannot use Cairo keywords see the full list at https://starknet.io/cairo-book/appendix-01-keywords.html consider renaming file: parent/hint.cairo"; "keyword")]
    fn validate_incorrect_test_target_name(name: &str) -> String {
        let source_path = Utf8PathBuf::from(format!("parent/{name}.cairo"));
        Target::validate_test_target_file_stem(source_path)
            .unwrap_err()
            .to_string()
    }
}
