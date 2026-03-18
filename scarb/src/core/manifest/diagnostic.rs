use crate::core::{PackageName, TargetKind};
use serde::Serialize;
use smol_str::SmolStr;
use toml_edit::{Document, Item, Table};

#[derive(Debug, Clone, Serialize)]
pub struct ManifestDiagnosticData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<ManifestDiagnosticSpan>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<ManifestRelatedLocation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManifestDiagnosticSpan {
    pub start: usize,
    pub end: usize,
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

#[derive(Debug, Clone)]
pub struct ManifestDiagnosticAnchor {
    target: ManifestAnchorTarget,
}

#[derive(Debug, Clone)]
pub enum ManifestDependencyTable {
    Dependencies,
    DevDependencies,
    WorkspaceDependencies,
}

#[derive(Debug, Clone)]
enum ManifestAnchorTarget {
    PackageField {
        field: &'static str,
    },
    WorkspacePackageField {
        field: &'static str,
    },
    Dependency {
        table: ManifestDependencyTable,
        name: PackageName,
        field: Option<&'static str>,
    },
    Profile {
        name: SmolStr,
        field: Option<&'static str>,
    },
    PatchRoot,
    PatchSource {
        source: SmolStr,
    },
    PatchDependency {
        source: SmolStr,
        name: PackageName,
        field: Option<&'static str>,
    },
    Target {
        kind: TargetKind,
        name: Option<SmolStr>,
        field: Option<&'static str>,
    },
}

impl ManifestDiagnosticAnchor {
    pub fn package_field(field: &'static str) -> Self {
        Self {
            target: ManifestAnchorTarget::PackageField { field },
        }
    }

    pub fn workspace_package_field(field: &'static str) -> Self {
        Self {
            target: ManifestAnchorTarget::WorkspacePackageField { field },
        }
    }

    pub fn dependency(table: ManifestDependencyTable, name: PackageName) -> Self {
        Self {
            target: ManifestAnchorTarget::Dependency {
                table,
                name,
                field: None,
            },
        }
    }

    pub fn profile(name: impl Into<SmolStr>) -> Self {
        Self {
            target: ManifestAnchorTarget::Profile {
                name: name.into(),
                field: None,
            },
        }
    }

    pub fn patch_root() -> Self {
        Self {
            target: ManifestAnchorTarget::PatchRoot,
        }
    }

    pub fn patch_source(source: impl Into<SmolStr>) -> Self {
        Self {
            target: ManifestAnchorTarget::PatchSource {
                source: source.into(),
            },
        }
    }

    pub fn patch_dependency(source: impl Into<SmolStr>, name: PackageName) -> Self {
        Self {
            target: ManifestAnchorTarget::PatchDependency {
                source: source.into(),
                name,
                field: None,
            },
        }
    }

    pub fn target(kind: TargetKind, name: Option<SmolStr>) -> Self {
        Self {
            target: ManifestAnchorTarget::Target {
                kind,
                name,
                field: None,
            },
        }
    }

    pub fn with_field(mut self, field: &'static str) -> Self {
        match &mut self.target {
            ManifestAnchorTarget::Dependency { field: f, .. }
            | ManifestAnchorTarget::Profile { field: f, .. }
            | ManifestAnchorTarget::PatchDependency { field: f, .. }
            | ManifestAnchorTarget::Target { field: f, .. } => {
                *f = Some(field);
            }
            _ => {}
        }
        self
    }
}

fn dependency_table_path(table: &ManifestDependencyTable) -> &'static [&'static str] {
    match table {
        ManifestDependencyTable::Dependencies => &["dependencies"],
        ManifestDependencyTable::DevDependencies => &["dev-dependencies"],
        ManifestDependencyTable::WorkspaceDependencies => &["workspace", "dependencies"],
    }
}

fn span_from_range(range: std::ops::Range<usize>) -> ManifestDiagnosticSpan {
    ManifestDiagnosticSpan {
        start: range.start,
        end: range.end,
    }
}

fn item_span(item: &Item) -> Option<ManifestDiagnosticSpan> {
    item.span().map(span_from_range)
}

fn table_span(table: &Table) -> Option<ManifestDiagnosticSpan> {
    table.span().map(span_from_range)
}

fn key_or_item_span(table: &Table, key: &str) -> Option<ManifestDiagnosticSpan> {
    let (key, item) = table.get_key_value(key)?;
    key.span().map(span_from_range).or_else(|| item_span(item))
}

fn key_or_item_span_in_inline_table(
    inline_table: &toml_edit::InlineTable,
    key: &str,
) -> Option<ManifestDiagnosticSpan> {
    let (key, item) = inline_table.get_key_value(key)?;
    key.span().map(span_from_range).or_else(|| item_span(item))
}

fn field_span_in_item(item: &Item, field: &str) -> Option<ManifestDiagnosticSpan> {
    if let Some(inline_table) = item.as_inline_table() {
        return key_or_item_span_in_inline_table(inline_table, field);
    }

    item.as_table()
        .and_then(|table| key_or_item_span(table, field))
}

fn table_entry_span(
    table: &Table,
    key: &str,
    field: Option<&'static str>,
) -> Option<ManifestDiagnosticSpan> {
    let (entry_key, entry_item) = table.get_key_value(key)?;
    let entry_span = entry_key
        .span()
        .map(span_from_range)
        .or_else(|| item_span(entry_item));

    if let Some(field) = field {
        return field_span_in_item(entry_item, field).or(entry_span);
    }

    entry_span
}

fn table_at_path<'a>(root: &'a Table, path: &[&str]) -> Option<&'a Table> {
    let mut current = root;
    for segment in path {
        current = current.get(segment)?.as_table()?;
    }
    Some(current)
}

fn patch_root_span(root: &Table) -> Option<ManifestDiagnosticSpan> {
    let patch = table_at_path(root, &["patch"])?;
    if !patch.is_implicit() {
        return table_span(patch);
    }

    patch.iter().find_map(|(_, item)| match item {
        Item::Table(table) => table_span(table),
        Item::ArrayOfTables(array) => array.iter().find_map(table_span),
        _ => None,
    })
}

fn patch_source_table<'a>(root: &'a Table, source: &str) -> Option<&'a Table> {
    table_at_path(root, &["patch"])?.get(source)?.as_table()
}

fn collect_target_candidates<'a>(root: &'a Table, kind: &TargetKind) -> Vec<&'a Table> {
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

    let mut candidates = Vec::new();
    let key = kind.as_str();
    push_item_candidates(root.get(key), &mut candidates);

    if let Some(target_table) = table_at_path(root, &["target"]) {
        push_item_candidates(target_table.get(key), &mut candidates);
    }

    candidates
}

fn target_name_matches(table: &Table, expected_name: &str) -> bool {
    table.get("name").and_then(Item::as_str) == Some(expected_name)
}

pub fn resolve_manifest_anchor(
    source: &str,
    anchor: &ManifestDiagnosticAnchor,
) -> Option<ManifestDiagnosticSpan> {
    let document = Document::parse(source).ok()?;
    resolve_anchor_in_doc(document.as_table(), anchor)
}

pub(crate) fn resolve_anchor_in_doc(
    root: &Table,
    anchor: &ManifestDiagnosticAnchor,
) -> Option<ManifestDiagnosticSpan> {
    match &anchor.target {
        ManifestAnchorTarget::PackageField { field } => {
            table_at_path(root, &["package"]).and_then(|table| key_or_item_span(table, field))
        }
        ManifestAnchorTarget::WorkspacePackageField { field } => {
            table_at_path(root, &["workspace", "package"])
                .and_then(|table| key_or_item_span(table, field))
        }
        ManifestAnchorTarget::Dependency { table, name, field } => {
            table_at_path(root, dependency_table_path(table)).and_then(|table| {
                table_entry_span(table, name.as_str(), *field)
                    .or_else(|| field.and_then(|field| key_or_item_span(table, field)))
            })
        }
        ManifestAnchorTarget::Profile { name, field } => {
            table_at_path(root, &["profile", name.as_str()]).and_then(|table| {
                field
                    .and_then(|field| key_or_item_span(table, field))
                    .or_else(|| table_span(table))
            })
        }
        ManifestAnchorTarget::PatchRoot => patch_root_span(root),
        ManifestAnchorTarget::PatchSource {
            source: patch_source,
        } => patch_source_table(root, patch_source.as_str()).and_then(table_span),
        ManifestAnchorTarget::PatchDependency {
            source: patch_source,
            name,
            field,
        } => patch_source_table(root, patch_source.as_str()).and_then(|table| {
            table_entry_span(table, name.as_str(), *field)
                .or_else(|| field.and_then(|field| key_or_item_span(table, field)))
        }),
        ManifestAnchorTarget::Target { kind, name, field } => {
            let candidates = collect_target_candidates(root, kind);
            let section = if let Some(name) = name.as_deref() {
                candidates
                    .iter()
                    .copied()
                    .find(|table| target_name_matches(table, name))
                    .or_else(|| candidates.first().copied())?
            } else {
                candidates.first().copied()?
            };

            if let Some(field) = field {
                return key_or_item_span(section, field).or_else(|| table_span(section));
            }

            if let Some(name) = name.as_deref() {
                if target_name_matches(section, name) {
                    if let Some(name_span) = key_or_item_span(section, "name") {
                        return Some(name_span);
                    }
                }
            }

            table_span(section)
        }
    }
}
