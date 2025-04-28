use crate::args::Args;
use anyhow::Result;
use mdbook::MDBook;
use scarb_ui::Ui;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub mod args;

pub fn main_inner(args: &Args, _ui: Ui) -> Result<()> {
    let mut book = MDBook::load(args.input.clone())?;
    let output_path: PathBuf = args.output.clone().into();
    book.config.build.build_dir = output_path
        .strip_prefix(&args.input)
        .unwrap_or(&output_path.clone())
        .into();
    book.build()?;
    let highlight = include_str!("../theme/highlight.js");
    let mut highlight_file = File::create(output_path.join("highlight.js"))?;
    highlight_file.write_all(highlight.as_bytes())?;
    Ok(())
}
