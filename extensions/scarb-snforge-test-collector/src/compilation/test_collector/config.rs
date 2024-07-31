use crate::felt252::Felt252;
use anyhow::Result;
use cairo_lang_defs::plugin::PluginDiagnostic;
use cairo_lang_diagnostics::Severity;
use cairo_lang_syntax::attribute::structured::{Attribute, AttributeArg, AttributeArgVariant};
use cairo_lang_syntax::node::ast::{ArgClause, Expr, PathSegment};
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::helpers::GetIdentifier;
use cairo_lang_syntax::node::TypedStablePtr;
use cairo_lang_test_plugin::test_config::{PanicExpectation, TestExpectation};
use cairo_lang_test_plugin::{try_extract_test_config, TestConfig};
use cairo_lang_utils::OptionHelper;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use serde::Serialize;
use std::num::NonZeroU32;

const FORK_ATTR: &str = "fork";
const FUZZER_ATTR: &str = "fuzzer";
const AVAILABLE_GAS_ATTR: &str = "available_gas";
/// Expectation for a panic case.
#[derive(Debug, Clone, PartialEq, Serialize)]
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
            PanicExpectation::Exact(vec) => {
                ExpectedPanicValue::Exact(vec.into_iter().map(Felt252::new).collect())
            }
        }
    }
}

/// Expectation for a result of a test.
#[derive(Debug, Clone, PartialEq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum RawForkConfig {
    Id(String),
    Params(RawForkParams),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RawForkParams {
    pub url: String,
    pub block_id_type: String,
    pub block_id_value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FuzzerConfig {
    pub fuzzer_runs: NonZeroU32,
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

/// Extracts the configuration of a tests from attributes, or returns the diagnostics if the
/// attributes are set illegally.
pub fn forge_try_extract_test_config(
    db: &dyn SyntaxGroup,
    attrs: &[Attribute],
) -> Result<Option<SingleTestConfig>, Vec<PluginDiagnostic>> {
    let maybe_test_config = try_extract_test_config(db, attrs.to_vec())?;
    let fork_attr = attrs.iter().find(|attr| attr.id.as_str() == FORK_ATTR);
    let fuzzer_attr = attrs.iter().find(|attr| attr.id.as_str() == FUZZER_ATTR);

    let mut diagnostics = vec![];

    if maybe_test_config.is_none() {
        for attr in [fork_attr, fuzzer_attr].into_iter().flatten() {
            diagnostics.push(PluginDiagnostic {
                severity: Severity::Error,
                stable_ptr: attr.id_stable_ptr.untyped(),
                message: "Attribute should only appear on tests.".into(),
            });
        }
    }

    let fork_config = if let Some(attr) = fork_attr {
        if attr.args.is_empty() {
            None
        } else {
            extract_fork_config(db, attr).on_none(|| {
                diagnostics.push(PluginDiagnostic {
                    severity: Severity::Error,
                    stable_ptr: attr.args_stable_ptr.untyped(),
                    message: "Expected fork config must be of the form `url: <double quote \
                                  string>, block_id: <snforge_std::BlockId>`."
                        .into(),
                });
            })
        }
    } else {
        None
    };

    let fuzzer_config = if let Some(attr) = fuzzer_attr {
        extract_fuzzer_config(db, attr).on_none(|| {
            diagnostics.push(PluginDiagnostic {
                severity: Severity::Error,
                stable_ptr: attr.args_stable_ptr.untyped(),
                message:
                    "Expected fuzzer config must be of the form `runs: <NonZeroU32>, seed: <u64>`"
                        .into(),
            });
        })
    } else {
        None
    };

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let result = maybe_test_config.map(
        |TestConfig {
             mut available_gas,
             expectation,
             ignored,
         }| {
            // Older versions will crash if the default is passed through
            let available_gas_attr = attrs
                .iter()
                .find(|attr| attr.id.as_str() == AVAILABLE_GAS_ATTR);

            if available_gas_attr.is_none() {
                available_gas = None
            }

            SingleTestConfig {
                available_gas,
                expected_result: expectation.into(),
                ignored,
                fork_config,
                fuzzer_config,
            }
        },
    );
    Ok(result)
}

fn extract_fork_config(db: &dyn SyntaxGroup, attr: &Attribute) -> Option<RawForkConfig> {
    if attr.args.is_empty() {
        return None;
    }

    match &attr.args[0].variant {
        AttributeArgVariant::Unnamed(fork_id) => extract_fork_config_from_id(fork_id, db),
        _ => extract_fork_config_from_args(db, attr),
    }
}

fn extract_fuzzer_config(db: &dyn SyntaxGroup, attr: &Attribute) -> Option<FuzzerConfig> {
    let [AttributeArg {
        variant:
            AttributeArgVariant::Named {
                name: fuzzer_runs_name,
                value: fuzzer_runs,
                ..
            },
        ..
    }, AttributeArg {
        variant:
            AttributeArgVariant::Named {
                name: fuzzer_seed_name,
                value: fuzzer_seed,
                ..
            },
        ..
    }] = &attr.args[..]
    else {
        return None;
    };

    if fuzzer_runs_name.text.as_str() != "runs" || fuzzer_seed_name.text.as_str() != "seed" {
        return None;
    };

    let fuzzer_runs = extract_numeric_value(db, fuzzer_runs)?
        .to_u32()?
        .try_into()
        .ok()?;
    let fuzzer_seed = extract_numeric_value(db, fuzzer_seed)?.to_u64()?;

    Some(FuzzerConfig {
        fuzzer_runs,
        fuzzer_seed,
    })
}

fn extract_numeric_value(db: &dyn SyntaxGroup, expr: &Expr) -> Option<BigInt> {
    let Expr::Literal(literal) = expr else {
        return None;
    };

    literal.numeric_value(db)
}

fn extract_fork_config_from_id(id: &Expr, db: &dyn SyntaxGroup) -> Option<RawForkConfig> {
    let Expr::String(id_str) = id else {
        return None;
    };
    let id = id_str.string_value(db)?;

    Some(RawForkConfig::Id(id))
}

fn extract_fork_config_from_args(db: &dyn SyntaxGroup, attr: &Attribute) -> Option<RawForkConfig> {
    let [AttributeArg {
        variant:
            AttributeArgVariant::Named {
                name: url_arg_name,
                value: url,
                ..
            },
        ..
    }, AttributeArg {
        variant:
            AttributeArgVariant::Named {
                name: block_id_arg_name,
                value: block_id,
                ..
            },
        ..
    }] = &attr.args[..]
    else {
        return None;
    };

    if url_arg_name.text.as_str() != "url" {
        return None;
    }
    let Expr::String(url_str) = url else {
        return None;
    };
    let url = url_str.string_value(db)?;

    if block_id_arg_name.text.as_str() != "block_id" {
        return None;
    }
    let Expr::FunctionCall(block_id) = block_id else {
        return None;
    };

    let elements: Vec<String> = block_id
        .path(db)
        .elements(db)
        .iter()
        .map(|e| e.identifier(db).to_string())
        .collect();
    if !(elements.len() == 2
        && elements[0] == "BlockId"
        && ["Number", "Hash", "Tag"].contains(&elements[1].as_str()))
    {
        return None;
    }

    let block_id_type = elements[1].clone();

    let args = block_id.arguments(db).arguments(db).elements(db);
    let expr = match args.first()?.arg_clause(db) {
        ArgClause::Unnamed(unnamed_arg_clause) => Some(unnamed_arg_clause.value(db)),
        _ => None,
    }?;
    let block_id_value = try_get_block_id(db, &block_id_type, &expr)?;

    Some(RawForkConfig::Params(RawForkParams {
        url,
        block_id_type,
        block_id_value,
    }))
}

fn try_get_block_id(db: &dyn SyntaxGroup, block_id_type: &str, expr: &Expr) -> Option<String> {
    match block_id_type {
        "Number" => {
            if let Expr::Literal(value) = expr {
                return Some(
                    u64::try_from(value.numeric_value(db).unwrap())
                        .ok()?
                        .to_string(),
                );
            }
        }
        "Hash" => {
            // TODO #1179: add range check
            if let Expr::Literal(value) = expr {
                return Some(value.numeric_value(db).unwrap().to_string());
            }
        }
        "Tag" => {
            if let Expr::Path(block_tag) = expr {
                let tag_elements = block_tag.elements(db);
                if tag_path_is_valid(&tag_elements, db) {
                    return Some("Latest".to_string());
                }
            }
        }
        _ => (),
    };

    None
}

// Only valid options are `BlockTag::Latest` and `Latest`
fn tag_path_is_valid(tag_elements: &[PathSegment], db: &dyn SyntaxGroup) -> bool {
    (tag_elements.len() == 1
        || (tag_elements.len() == 2 && tag_elements[0].identifier(db).as_str() == "BlockTag"))
        && tag_elements.last().unwrap().identifier(db).as_str() == "Latest"
}
