use console::Style;
use serde::{Serialize, Serializer};

use crate::Message;

/// Result of a single test execution.
///
/// Displays as `test {name} ... {status}` where `status` is colored:
/// - `ok` in green
/// - `FAILED` in red
/// - `ignored` in yellow
#[derive(Serialize)]
// TODO: move this to `scarb-doc`
pub struct TestResult<'a> {
    name: &'a str,
    #[serde(skip)]
    status: TestResultStatus,
}

/// Status of a test result.
#[derive(Clone, Copy)]
pub enum TestResultStatus {
    /// Test passed.
    Ok,
    /// Test failed.
    Failed,
    /// Test was ignored.
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
    /// Creates a new [`TestResult`].
    pub fn new(name: &'a str, status: TestResultStatus) -> Self {
        Self { name, status }
    }

    /// Creates a new `ok` test result.
    pub fn ok(name: &'a str) -> Self {
        Self::new(name, TestResultStatus::Ok)
    }

    /// Creates a new `FAILED` test result.
    pub fn failed(name: &'a str) -> Self {
        Self::new(name, TestResultStatus::Failed)
    }

    /// Creates a new `ignored` test result.
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
