use cairo_felt::Felt252;
use cairo_lang_test_plugin::test_config::{PanicExpectation, TestExpectation};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum CrateLocation {
    /// Main crate in a package
    Lib,
    /// Crate in the `tests/` directory
    Tests,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct TestCaseRaw {
    pub name: String,
    pub available_gas: Option<usize>,
    pub ignored: bool,
    pub expected_result: ExpectedTestResult,
    pub fork_config: Option<RawForkConfig>,
    pub fuzzer_config: Option<FuzzerConfig>,
}

/// Expectation for a panic case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExpectedPanicValue {
    /// Accept any panic value.
    Any,
    /// Accept only this specific vector of panics.
    Exact(Vec<Felt252>),
}

impl From<PanicExpectation> for ExpectedPanicValue {
    fn from(value: PanicExpectation) -> Self {
        match value {
            PanicExpectation::Any => ExpectedPanicValue::Any,
            PanicExpectation::Exact(vec) => ExpectedPanicValue::Exact(vec),
        }
    }
}

/// Expectation for a result of a test.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExpectedTestResult {
    /// Running the test should not panic.
    Success,
    /// Running the test should result in a panic.
    Panics(ExpectedPanicValue),
}

impl From<TestExpectation> for ExpectedTestResult {
    fn from(value: TestExpectation) -> Self {
        match value {
            TestExpectation::Success => ExpectedTestResult::Success,
            TestExpectation::Panics(panic_expectation) => {
                ExpectedTestResult::Panics(panic_expectation.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuzzerConfig {
    pub fuzzer_runs: u32,
    pub fuzzer_seed: u64,
}

/// The configuration for running a single test.
#[derive(Debug)]
pub struct SingleTestConfig {
    /// The amount of gas the test requested.
    pub available_gas: Option<usize>,
    /// The expected result of the run.
    pub expected_result: ExpectedTestResult,
    /// Should the test be ignored.
    pub ignored: bool,
    /// The configuration of forked network.
    pub fork_config: Option<RawForkConfig>,
    /// Custom fuzzing configuration
    pub fuzzer_config: Option<FuzzerConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RawForkConfig {
    Id(String),
    Params(RawForkParams),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawForkParams {
    pub url: String,
    pub block_id_type: String,
    pub block_id_value: String,
}
