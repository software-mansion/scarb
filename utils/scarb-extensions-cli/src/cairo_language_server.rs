#![deny(missing_docs)]

//! Extension CLI arguments datastructures.

use clap::Parser;

/// CLI command name.
pub const COMMAND_NAME: &str = "cairo-language-server";

/// Start the Cairo Language Server
#[derive(Parser, Clone, Debug)]
#[clap(name = COMMAND_NAME, verbatim_doc_comment, disable_help_flag = true, disable_version_flag = true)]
pub struct Args {}
