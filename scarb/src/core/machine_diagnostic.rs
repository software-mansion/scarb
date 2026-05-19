use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct MachineDiagnostic {
    pub kind: MachineDiagnosticKind,
    pub message: String,
    pub file: String,
    pub span: MachineDiagnosticSpan,
    pub severity: MachineDiagnosticSeverity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<MachineRelatedLocation>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MachineDiagnosticKind {
    Diagnostic,
    ManifestDiagnostic,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MachineDiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct MachineDiagnosticSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct MachineRelatedLocation {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub span: MachineDiagnosticSpan,
}

#[derive(Debug, Clone, Serialize)]
pub struct MachineDiagnosticData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<MachineDiagnosticSpan>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<MachineRelatedLocation>,
}

impl MachineDiagnostic {
    pub fn new(
        kind: MachineDiagnosticKind,
        message: String,
        severity: MachineDiagnosticSeverity,
        file: String,
        span: MachineDiagnosticSpan,
    ) -> Self {
        Self {
            kind,
            message,
            severity,
            code: None,
            file,
            span,
            related: vec![],
        }
    }

    pub fn severity(&self) -> MachineDiagnosticSeverity {
        self.severity
    }
}

impl From<std::ops::Range<usize>> for MachineDiagnosticSpan {
    fn from(range: std::ops::Range<usize>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}
