use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use cairo_lang_formatter::{CairoFormatter, FormatOutcome, FormatterConfig};
use ignore::WalkState::{Continue, Skip};
use ignore::{DirEntry, Error, ParallelVisitor, ParallelVisitorBuilder, WalkState};
use tracing::{info, warn};

use crate::core::workspace::Workspace;
use crate::core::PackageId;

#[derive(Debug)]
pub struct FmtOptions {
    pub check: bool,
    pub packages: Vec<PackageId>,
    pub color: bool,
}

#[tracing::instrument(skip_all, level = "debug")]
pub fn format(opts: FmtOptions, ws: &Workspace<'_>) -> Result<bool> {
    ws.config().ui().force_colors_enabled(opts.color);
    let config = FormatterConfig::default();
    let fmt = CairoFormatter::new(config);
    let packages = ws
        .members()
        .filter(|pkg| opts.packages.contains(&pkg.id))
        .collect::<Vec<_>>();
    let Some((first_package, packages)) = packages.split_first() else {
        return Ok(true);
    };
    let mut walk = fmt.walk(first_package.root().as_std_path());
    for package in packages {
        walk.add(package.root().as_std_path());
    }
    let all_correct = AtomicBool::new(true);
    let mut builder = PathFormatterBuilder {
        ws,
        fmt: &fmt,
        opts: &opts,
        all_correct: &all_correct,
        selected_packages: opts.packages.clone(),
    };
    walk.build_parallel().visit(&mut builder);
    let result = builder.all_correct.load(Ordering::Acquire);

    Ok(result)
}

struct PathFormatter<'t> {
    all_correct: &'t AtomicBool,
    opts: &'t FmtOptions,
    fmt: &'t CairoFormatter,
    ws: &'t Workspace<'t>,
    skip: Vec<PathBuf>,
}

struct PathFormatterBuilder<'t> {
    selected_packages: Vec<PackageId>,
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
        let skip = self
            .ws
            .members()
            .filter(|pkg| !self.selected_packages.contains(&pkg.id))
            .map(|pkg| pkg.root().as_std_path().to_path_buf())
            .collect::<Vec<_>>();
        Box::new(PathFormatter {
            all_correct: self.all_correct,
            opts: self.opts,
            fmt: self.fmt,
            ws: self.ws,
            skip,
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

        let path = dir_entry.path();

        if file_type.is_dir() && self.skip.contains(&path.to_path_buf()) {
            // Ignore workspace members that are not selected with package filter.
            return Skip;
        }

        if !file_type.is_file() {
            return Continue;
        }

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
