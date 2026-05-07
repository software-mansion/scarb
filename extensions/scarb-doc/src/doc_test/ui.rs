use crate::doc_test::runner::TestSummary;
use console::Style;
use scarb_ui::Message;
use serde::{Serialize, Serializer};
use std::fmt;

impl Message for TestSummary {
    fn text(self) -> String {
        let status = if self.is_ok() {
            TestResultStatus::Ok
        } else {
            TestResultStatus::Failed
        };
        format!(
            "test result: {}. {} passed; {} failed; {} ignored",
            status.style().apply_to(status.to_string()),
            self.passed,
            self.failed,
            self.ignored
        )
    }

    fn structured<S: Serializer>(self, ser: S) -> anyhow::Result<S::Ok, S::Error> {
        self.serialize(ser)
    }
}

#[derive(Serialize)]
pub enum TestResultStatus {
    Ok,
    Failed,
    Ignored,
}

impl TestResultStatus {
    pub fn style(&self) -> Style {
        match self {
            TestResultStatus::Ok => Style::new().green(),
            TestResultStatus::Failed => Style::new().red(),
            TestResultStatus::Ignored => Style::new().yellow(),
        }
    }

    pub fn display_for(self, name: &str) -> String {
        format!(
            "test {} ... {}",
            name,
            self.style().apply_to(self.to_string())
        )
    }
}

impl fmt::Display for TestResultStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TestResultStatus::Ok => "ok",
            TestResultStatus::Failed => "FAILED",
            TestResultStatus::Ignored => "ignored",
        };
        write!(f, "{s}")
    }
}
