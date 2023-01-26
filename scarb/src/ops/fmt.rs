use anyhow::Result;
use cairo_lang_formatter::{CairoFormatter, FormatOutcome, FormatterConfig};
use ignore::WalkState::Continue;
use ignore::{DirEntry, Error, ParallelVisitor, ParallelVisitorBuilder, WalkState};
use std::fmt::Display;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{info, warn};

use crate::core::workspace::Workspace;
use crate::core::{Package, PackageName};

#[derive(Debug)]
pub struct FmtOptions {
    pub check: bool,
    pub pkg_name: Option<PackageName>,
    pub color: bool,
}

#[tracing::instrument(skip_all, level = "debug")]
pub fn format(opts: FmtOptions, ws: &Workspace<'_>) -> Result<bool> {
    console::set_colors_enabled(opts.color);
    if let Some(pkg_name) = opts.pkg_name.clone() {
        // Format single package by name.
        format_package_by_cond(ws, &opts, &|pkg: &Package| pkg.id.name == pkg_name)
    } else {
        // Format project members.
        format_package_by_cond(ws, &opts, &|_pkg: &Package| true)
    }
}

struct PathFormatter<'t> {
    all_correct: &'t AtomicBool,
    opts: &'t FmtOptions,
    fmt: &'t CairoFormatter,
    ws: &'t Workspace<'t>,
}

struct PathFormatterBuilder<'t> {
    all_correct: &'t AtomicBool,
    opts: &'t FmtOptions,
    fmt: &'t CairoFormatter,
    ws: &'t Workspace<'t>,
}

impl<'s, 't> ParallelVisitorBuilder<'s> for PathFormatterBuilder<'t>
where
    't: 's,
{
    fn build(&mut self) -> Box<dyn ParallelVisitor + 's> {
        Box::new(PathFormatter {
            all_correct: self.all_correct,
            opts: self.opts,
            fmt: self.fmt,
            ws: self.ws,
        })
    }
}

fn print_diff(ws: &Workspace<'_>, path: &Path, diff: impl Display) {
    ws.config()
        .ui()
        .print(format!("Diff in file {}:\n {}", path.display(), diff));
}

fn print_error(ws: &Workspace<'_>, path: &Path, error: anyhow::Error) {
    let error_msg = error.to_string();
    ws.config().ui().error(format!(
        "{}Error writing files: cannot parse {}",
        // TODO(maciektr): Fix this with proper upstream changes.
        //   The slice is a hacky way of avoiding duplicated "error: " prefix.
        &error_msg[7..],
        path.display()
    ));
}

fn check_file_formatting(
    fmt: &CairoFormatter,
    opts: &FmtOptions,
    ws: &Workspace<'_>,
    path: &Path,
) -> bool {
    match fmt.format_to_string(&path) {
        Ok(FormatOutcome::Identical(_)) => true,
        Ok(FormatOutcome::DiffFound(diff)) => {
            if opts.color {
                print_diff(ws, path, diff.display_colored());
            } else {
                print_diff(ws, path, diff);
            }

            false
        }
        Err(parsing_error) => {
            print_error(ws, path, parsing_error);
            false
        }
    }
}

fn format_file_in_place(
    fmt: &CairoFormatter,
    _opts: &FmtOptions,
    ws: &Workspace<'_>,
    path: &Path,
) -> bool {
    if let Err(parsing_error) = fmt.format_in_place(&path) {
        print_error(ws, path, parsing_error);
        false
    } else {
        true
    }
}

impl<'t> ParallelVisitor for PathFormatter<'t> {
    fn visit(&mut self, dir_entry_res: Result<DirEntry, Error>) -> WalkState {
        let dir_entry = if let Ok(dir_entry) = dir_entry_res {
            dir_entry
        } else {
            warn!("Failed to read the file.");
            return Continue;
        };

        let file_type = if let Some(file_type) = dir_entry.file_type() {
            file_type
        } else {
            warn!("Failed to read filetype.");
            return Continue;
        };

        if !file_type.is_file() {
            return Continue;
        }

        let path = dir_entry.path();
        info!("Formatting file: {}.", path.display());

        let success = if self.opts.check {
            check_file_formatting(self.fmt, self.opts, self.ws, path)
        } else {
            format_file_in_place(self.fmt, self.opts, self.ws, path)
        };

        if !success {
            self.all_correct.store(false, Ordering::Release);
        }

        Continue
    }
}

fn format_package(opts: &FmtOptions, pkg: &Package, ws: &Workspace<'_>) -> Result<bool> {
    let config = FormatterConfig::default();
    let fmt = CairoFormatter::new(config);

    let base_path = Path::new(pkg.manifest_path().parent().unwrap());
    let walk = fmt.walk(base_path);
    let all_correct = AtomicBool::new(true);
    let mut builder = PathFormatterBuilder {
        ws,
        opts,
        fmt: &fmt,
        all_correct: &all_correct,
    };
    walk.build_parallel().visit(&mut builder);
    let result = builder.all_correct.load(Ordering::Acquire);

    Ok(result)
}

fn format_package_by_cond(
    ws: &Workspace<'_>,
    opts: &FmtOptions,
    cond: &dyn Fn(&Package) -> bool,
) -> Result<bool> {
    let mut result = true;

    for pkg in ws.members() {
        if cond(&pkg) {
            result &= format_package(opts, &pkg, ws)?;
        }
    }

    Ok(result)
}
