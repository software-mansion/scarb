use crate::doc_test::runner::TestSummary;
use console::Style;
use scarb_ui::Message;
use serde::{Serialize, Serializer};

impl Message for TestSummary {
    fn text(self) -> String {
        let (result, style) = if self.is_ok() {
            ("ok", Style::new().green())
        } else {
            ("FAILED", Style::new().red())
        };
        format!(
            "test result: {}. {} passed; {} failed; {} ignored",
            style.apply_to(result),
            self.passed,
            self.failed,
            self.ignored
        )
    }

    fn structured<S: Serializer>(self, ser: S) -> anyhow::Result<S::Ok, S::Error> {
        self.serialize(ser)
    }
}

/// Result of a single test execution.
///
/// Displays as `test {name} ... {status}` where `status` is colored:
/// - `ok` in green
/// - `FAILED` in red
/// - `ignored` in yellow
#[derive(Serialize)]
pub struct TestResult<'a> {
    name: &'a str,
    #[serde(skip)]
    status: TestResultStatus,
}

#[derive(Clone, Copy)]
pub enum TestResultStatus {
    Ok,
    Failed,
    Ignored,
}

impl Serialize for TestResultStatus {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(self.as_str())
    }
}

impl TestResultStatus {
    fn as_str(&self) -> &'static str {
        match self {
            TestResultStatus::Ok => "ok",
            TestResultStatus::Failed => "FAILED",
            TestResultStatus::Ignored => "ignored",
        }
    }

    fn style(&self) -> Style {
        match self {
            TestResultStatus::Ok => Style::new().green(),
            TestResultStatus::Failed => Style::new().red(),
            TestResultStatus::Ignored => Style::new().yellow(),
        }
    }
}

impl<'a> TestResult<'a> {
    pub fn new(name: &'a str, status: TestResultStatus) -> Self {
        Self { name, status }
    }

    pub fn ok(name: &'a str) -> Self {
        Self::new(name, TestResultStatus::Ok)
    }

    pub fn failed(name: &'a str) -> Self {
        Self::new(name, TestResultStatus::Failed)
    }

    pub fn ignored(name: &'a str) -> Self {
        Self::new(name, TestResultStatus::Ignored)
    }
}

impl Message for TestResult<'_> {
    fn text(self) -> String {
        format!(
            "test {} ... {}",
            self.name,
            self.status.style().apply_to(self.status.as_str())
        )
    }

    fn structured<S: Serializer>(self, ser: S) -> Result<S::Ok, S::Error> {
        TestResultJson {
            r#type: "test_result",
            name: self.name,
            status: self.status,
        }
        .serialize(ser)
    }
}

#[derive(Serialize)]
struct TestResultJson<'a> {
    r#type: &'static str,
    name: &'a str,
    status: TestResultStatus,
}
