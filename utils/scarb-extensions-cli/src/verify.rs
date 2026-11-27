#![deny(missing_docs)]

//! Extension CLI arguments datastructures.

use camino::Utf8PathBuf;
use clap::Parser;
use scarb_ui::args::{PackagesFilter, VerbositySpec};

/// CLI command name.
pub const COMMAND_NAME: &str = "verify";

/// Verify `scarb prove` output using Stwo verifier
#[derive(Parser, Clone, Debug)]
#[clap(name = COMMAND_NAME, version, verbatim_doc_comment)]
pub struct Args {
    /// Name of the package.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// ID of `scarb execute` output for given package, for which proof was generated using `scarb prove`.
    #[arg(long)]
    pub execution_id: Option<u32>,

    /// Proof file path.
    #[arg(
        long,
        required_unless_present = "execution_id",
        conflicts_with = "execution_id"
    )]
    pub proof_file: Option<Utf8PathBuf>,

    /// Print machine-readable output in NDJSON format.
    #[arg(long, env = "SCARB_OUTPUT_JSON")]
    pub json: bool,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,
}

impl Args {
    /// Construct [`scarb_ui::OutputFormat`] value from these arguments.
    pub fn output_format(&self) -> scarb_ui::OutputFormat {
        if self.json {
            scarb_ui::OutputFormat::Json
        } else {
            scarb_ui::OutputFormat::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Args;
    use clap::Parser;

    const DOCS: &str = include_str!("../../../website/docs/extensions/prove-and-verify.md");

    fn expand_placeholders(input: &str) -> String {
        let mut result = String::new();
        let mut rest = input;
        while let Some(start) = rest.find('<') {
            result.push_str(&rest[..start]);
            match rest[start + 1..].find('>') {
                Some(rel_end) => {
                    let inner = &rest[start + 1..start + 1 + rel_end];
                    let replacement = if inner.contains("path") || inner.contains("file") {
                        "nonexistent.json"
                    } else {
                        "1"
                    };
                    result.push_str(replacement);
                    rest = &rest[start + 2 + rel_end..];
                }
                None => {
                    result.push_str(rest);
                    rest = "";
                    break;
                }
            }
        }
        result.push_str(rest);
        result
    }

    fn documented_verify_args() -> Vec<Vec<String>> {
        let mut commands = Vec::new();
        let mut in_fence = false;
        for line in DOCS.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("```") {
                in_fence = !in_fence;
                continue;
            }
            if in_fence && trimmed.starts_with("scarb verify") {
                let expanded = expand_placeholders(trimmed);
                let mut args = vec!["scarb-verify".to_string()];
                args.extend(
                    expanded
                        .split_whitespace()
                        .skip_while(|token| *token == "scarb" || *token == "verify")
                        .map(str::to_string),
                );
                commands.push(args);
            }
        }
        commands
    }

    #[test]
    fn documented_verify_invocations_parse() {
        let commands = documented_verify_args();
        assert!(
            !commands.is_empty(),
            "expected at least one `scarb verify` example in prove-and-verify.md"
        );
        for args in commands {
            Args::try_parse_from(&args)
                .unwrap_or_else(|e| panic!("documented invocation failed to parse: {args:?}\n{e}"));
        }
    }
}
