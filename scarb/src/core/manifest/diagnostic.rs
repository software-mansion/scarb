use crate::core::{PackageName, TargetKind};
use serde::Serialize;
use toml_edit::{Document, Item, Table};

#[derive(Debug, Clone, Serialize)]
pub struct ManifestDiagnosticData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<ManifestDiagnosticSpan>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<ManifestRelatedLocation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManifestRelatedLocation {
    pub message: String,
    pub span: ManifestDiagnosticSpan,
}

#[derive(Debug, Clone)]
pub struct ManifestRelatedAnchor {
    pub message: String,
    pub anchor: ManifestDiagnosticAnchor,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManifestDiagnosticSpan {
    pub start: usize,
    pub end: usize,
}

impl From<std::ops::Range<usize>> for ManifestDiagnosticSpan {
    fn from(range: std::ops::Range<usize>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ManifestDependencyTable {
    Dependencies,
    DevDependencies,
    WorkspaceDependencies,
}

impl ManifestDependencyTable {
    /// Returns the TOML key path for this dependency table,
    /// e.g. `["dependencies"]` or `["workspace", "dependencies"]`.
    pub fn path(&self) -> &'static [&'static str] {
        match self {
            Self::Dependencies => &["dependencies"],
            Self::DevDependencies => &["dev-dependencies"],
            Self::WorkspaceDependencies => &["workspace", "dependencies"],
        }
    }
}

#[derive(Debug, Clone)]
pub enum ManifestDiagnosticAnchor {
    /// A field under `[package]`, e.g. `package.readme`.
    PackageField { field: &'static str },
    /// A field under `[workspace.package]`, e.g. `workspace.package.version`.
    WorkspacePackageField { field: &'static str },
    /// A dependency entry in a dependency table, e.g. `[dependencies].starknet`.
    Dependency {
        table: ManifestDependencyTable,
        name: PackageName,
        field: Option<&'static str>,
    },
    /// A profile section or field, e.g. `[profile.release].inherits`.
    Profile {
        name: String,
        field: Option<&'static str>,
    },
    /// The root `[patch]` section itself.
    PatchRoot,
    /// A patch source table, e.g. `[patch.crates-io]`.
    PatchSource { source: String },
    /// A dependency inside a patch source, e.g. `[patch.crates-io].foo`.
    PatchDependency {
        source: String,
        name: PackageName,
        field: Option<&'static str>,
    },
    /// A target table or field, e.g. `[[target.starknet-contract]]` or its `name`.
    Target {
        kind: TargetKind,
        name: Option<String>,
        field: Option<&'static str>,
    },
}

impl ManifestDiagnosticAnchor {
    /// Targets `[package].<field>`.
    pub fn package_field(field: &'static str) -> Self {
        Self::PackageField { field }
    }

    /// Targets `[workspace.package].<field>`.
    pub fn workspace_package_field(field: &'static str) -> Self {
        Self::WorkspacePackageField { field }
    }

    /// Targets a dependency entry in the given table, e.g. `[dependencies].<name>`.
    pub fn dependency(table: ManifestDependencyTable, name: PackageName) -> Self {
        Self::Dependency {
            table,
            name,
            field: None,
        }
    }

    /// Targets a profile section, e.g. `[profile.<name>]`.
    pub fn profile(name: impl Into<String>) -> Self {
        Self::Profile {
            name: name.into(),
            field: None,
        }
    }

    /// Targets the `[patch]` section (or its first child if the header is implicit).
    pub fn patch_root() -> Self {
        Self::PatchRoot
    }

    /// Targets a patch source table, e.g. `[patch.<source>]`.
    pub fn patch_source(source: impl Into<String>) -> Self {
        Self::PatchSource {
            source: source.into(),
        }
    }

    /// Targets a specific dependency within a patch source, e.g. `[patch.<source>.<name>]`.
    pub fn patch_dependency(source: impl Into<String>, name: PackageName) -> Self {
        Self::PatchDependency {
            source: source.into(),
            name,
            field: None,
        }
    }

    /// Targets a target section, e.g. `[[lib]]` or `[[test]]`.
    pub fn target(kind: TargetKind, name: Option<String>) -> Self {
        Self::Target {
            kind,
            name,
            field: None,
        }
    }

    /// Refines the anchor to point at a specific field within the targeted table entry.
    pub fn with_field(mut self, field: &'static str) -> Self {
        match &mut self {
            Self::Dependency { field: f, .. }
            | Self::Profile { field: f, .. }
            | Self::PatchDependency { field: f, .. }
            | Self::Target { field: f, .. } => {
                *f = Some(field);
            }
            _ => panic!("Cannot create anchor to a field in a non-table entry"),
        }
        self
    }
}

/// Returns the span of `key` in `table`, preferring the key token span over its value's span.
/// Example: in `version = "1.0.0"`, resolves to `version` (or to `"1.0.0"` if the key span is unavailable).
fn key_or_item_span(table: &Table, key: &str) -> Option<ManifestDiagnosticSpan> {
    let (key, item) = table.get_key_value(key)?;
    key.span()
        .map(ManifestDiagnosticSpan::from)
        .or_else(|| item.span().map(ManifestDiagnosticSpan::from))
}

/// Returns the span of an entry in `table` by `key`.
/// Example: in `foo = { path = "../foo" }`, `foo` selects `foo`; with `field = Some("path")`, selects `path`.
/// Returns `None` if the entry or the requested field does not exist.
fn table_entry_span(
    table: &Table,
    key: &str,
    field: Option<&'static str>,
) -> Option<ManifestDiagnosticSpan> {
    let (entry_key, entry_item) = table.get_key_value(key)?;

    let Some(field) = field else {
        return entry_key
            .span()
            .map(ManifestDiagnosticSpan::from)
            .or_else(|| entry_item.span().map(ManifestDiagnosticSpan::from));
    };

    if let Some(inline_table) = entry_item.as_inline_table() {
        let (key, item) = inline_table.get_key_value(field)?;
        return key
            .span()
            .map(ManifestDiagnosticSpan::from)
            .or_else(|| item.span().map(ManifestDiagnosticSpan::from));
    }

    entry_item
        .as_table()
        .and_then(|table| key_or_item_span(table, field))
}

/// Walks a dotted TOML key path and returns the nested table at that location.
fn table_at_path<'a>(root: &'a Table, path: &[&str]) -> Option<&'a Table> {
    let mut current = root;
    for segment in path {
        current = current.get(segment)?.as_table()?;
    }
    Some(current)
}

/// Returns the `[patch.<source>]` table, if present.
fn patch_source_table<'a>(root: &'a Table, source: &str) -> Option<&'a Table> {
    table_at_path(root, &["patch"])?.get(source)?.as_table()
}

pub fn resolve_manifest_anchor(
    source: &str,
    anchor: &ManifestDiagnosticAnchor,
) -> Option<ManifestDiagnosticSpan> {
    let document = Document::parse(source).ok()?;
    resolve_anchor_in_doc(document.as_table(), anchor)
}

/// Resolves a high-level manifest anchor to the concrete TOML span to highlight.
pub fn resolve_anchor_in_doc(
    root: &Table,
    anchor: &ManifestDiagnosticAnchor,
) -> Option<ManifestDiagnosticSpan> {
    match anchor {
        ManifestDiagnosticAnchor::PackageField { field } => {
            table_at_path(root, &["package"]).and_then(|table| key_or_item_span(table, field))
        }
        ManifestDiagnosticAnchor::WorkspacePackageField { field } => {
            table_at_path(root, &["workspace", "package"])
                .and_then(|table| key_or_item_span(table, field))
        }
        ManifestDiagnosticAnchor::Dependency { table, name, field } => {
            table_at_path(root, table.path())
                .and_then(|table| table_entry_span(table, name.as_str(), *field))
        }
        ManifestDiagnosticAnchor::Profile { name, field } => {
            table_at_path(root, &["profile", name.as_str()]).and_then(|table| {
                field
                    .and_then(|field| key_or_item_span(table, field))
                    .or_else(|| table.span().map(ManifestDiagnosticSpan::from))
            })
        }
        ManifestDiagnosticAnchor::PatchRoot => {
            let patch = table_at_path(root, &["patch"])?;
            if !patch.is_implicit() {
                patch.span().map(ManifestDiagnosticSpan::from)
            } else {
                patch.iter().find_map(|(_, item)| match item {
                    Item::Table(table) => table.span().map(ManifestDiagnosticSpan::from),
                    _ => None,
                })
            }
        }
        ManifestDiagnosticAnchor::PatchSource {
            source: patch_source,
        } => patch_source_table(root, patch_source.as_str())
            .and_then(|t| t.span().map(ManifestDiagnosticSpan::from)),
        ManifestDiagnosticAnchor::PatchDependency {
            source: patch_source,
            name,
            field,
        } => patch_source_table(root, patch_source.as_str())
            .and_then(|table| table_entry_span(table, name.as_str(), *field)),
        ManifestDiagnosticAnchor::Target { kind, name, field } => {
            fn push_item_candidates<'a>(item: Option<&'a Item>, out: &mut Vec<&'a Table>) {
                let Some(item) = item else {
                    return;
                };
                match item {
                    Item::Table(table) => out.push(table),
                    Item::ArrayOfTables(array) => out.extend(array.iter()),
                    _ => {}
                }
            }

            // Contains target candidates with the given kind.
            let mut candidates = Vec::new();
            // Covers e.g. `[executable]` syntax.
            push_item_candidates(root.get(kind.as_str()), &mut candidates);

            // Covers e.g. `[[target.executable]]` syntax.
            if let Some(target_table) = table_at_path(root, &["target"]) {
                push_item_candidates(target_table.get(kind.as_str()), &mut candidates);
            }

            let section = if let Some(name) = name.as_deref() {
                candidates
                    .iter()
                    .copied()
                    .find(|table| table.get("name").and_then(Item::as_str) == Some(name))
                    .or_else(|| {
                        field
                            .is_none()
                            .then(|| candidates.first().copied())
                            .flatten()
                    })?
            } else {
                // Example: `[test]`; if there is only one candidate of this kind, it can still
                // be highlighted even without an explicit target name.
                (candidates.len() == 1)
                    .then(|| candidates.first().copied())
                    .flatten()?
            };

            if let Some(field) = field {
                return key_or_item_span(section, field);
            }

            // For named `[[target.<kind>]]` entries, highlight the `name` field instead of the
            // whole table, e.g. `[[target.executable]] name = "secondary"`.
            if let Some(name) = name.as_deref()
                && section.get("name").and_then(Item::as_str) == Some(name)
                && let Some(name_span) = key_or_item_span(section, "name")
            {
                return Some(name_span);
            }

            section.span().map(ManifestDiagnosticSpan::from)
        }
    }
}
