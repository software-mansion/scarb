use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use cairo_lang_diagnostics::Severity;
use cairo_lang_formatter::cairo_formatter::FormattingError;
use cairo_lang_formatter::{CairoFormatter, FormatOutcome, FormatterConfig};
use camino::Utf8PathBuf;
use ignore::WalkState::{Continue, Skip};
use ignore::{DirEntry, Error, ParallelVisitor, ParallelVisitorBuilder, WalkState};
use scarb_ui::Message;
use tracing::{info, warn};

use crate::core::workspace::Workspace;
use crate::core::{Package, PackageId};
use crate::internal::fsx::canonicalize;
use crate::internal::serdex::toml_merge;

#[derive(Debug, Clone)]
pub enum FmtEmitTarget {
    Stdout,
}

/// Format option to display the output formatted file
/// Example: scarb fmt --emit stdout
#[derive(Debug, Default, Clone)]
pub enum FmtAction {
    #[default]
    Fix,
    Check,
    Emit(FmtEmitTarget),
}

#[derive(Debug)]
pub struct FmtOptions {
    pub action: FmtAction,
    pub packages: Vec<PackageId>,
    pub color: bool,
    pub path: Option<Utf8PathBuf>,
}

#[tracing::instrument(skip_all, level = "debug")]
pub fn format(opts: FmtOptions, ws: &Workspace<'_>) -> Result<bool> {
    ws.config().ui().force_colors_enabled(opts.color);

    let all_correct = AtomicBool::new(true);

    if let Some(path) = &opts.path {
        if path.is_file() {
            format_single_file(path, &opts, ws, &all_correct)?;
            return Ok(all_correct.load(Ordering::Acquire));
        }

        let absolute_path = canonicalize(path)?;
        let pkg = ws
            .members()
            .find(|member| absolute_path.starts_with(member.root()));
        if let Some(pkg) = pkg {
            format_package(&pkg, &opts, ws, &all_correct)?;
            return Ok(all_correct.load(Ordering::Acquire));
        } else {
            anyhow::bail!("path {:?} is not part of the workspace.", path);
        };
    };
    for package_id in opts.packages.iter() {
        let package = ws.fetch_package(package_id)?;
        format_package(package, &opts, ws, &all_correct)?;
    }

    Ok(all_correct.load(Ordering::Acquire))
}

fn format_package(
    package: &Package,
    opts: &FmtOptions,
    ws: &Workspace<'_>,
    all_correct: &AtomicBool,
) -> Result<()> {
    let mut config = FormatterConfig::default();
    if let Some(overrides) = package.tool_metadata("fmt") {
        config = toml_merge(&config, overrides)?;
    }
    let fmt = CairoFormatter::new(config);

    let walk = if let Some(path) = &opts.path {
        fmt.walk(path.as_std_path())
    } else {
        fmt.walk(package.root().as_std_path())
    };

    let mut builder = PathFormatterBuilder {
        ws,
        fmt: &fmt,
        opts,
        all_correct,
        selected_packages: vec![package.id],
    };
    walk.build_parallel().visit(&mut builder);

    Ok(())
}

fn format_single_file(
    file: &Utf8PathBuf,
    opts: &FmtOptions,
    ws: &Workspace<'_>,
    all_correct: &AtomicBool,
) -> Result<bool> {
    let absolute_file_path = canonicalize(file)?;
    let pkg = ws
        .members()
        .find(|member| absolute_file_path.starts_with(member.root()))
        .ok_or_else(|| anyhow::anyhow!("file {:?} is not part of the workspace.", file))?;

    let mut config = FormatterConfig::default();
    if let Some(overrides) = pkg.tool_metadata("fmt") {
        config = toml_merge(&config, overrides)?;
    }
    let fmt = CairoFormatter::new(config);

    let success = match &opts.action {
        FmtAction::Fix => format_file_in_place(&fmt, opts, ws, absolute_file_path.as_path()),
        FmtAction::Check => check_file_formatting(&fmt, opts, ws, absolute_file_path.as_path()),
        FmtAction::Emit(target) => emit_formatted_file(
            &fmt,
            target,
            ws,
            absolute_file_path.as_path(),
            EmitMode::WithoutPath,
        ),
    };

    if !success {
        all_correct.store(false, Ordering::Release);
    }

    Ok(all_correct.load(Ordering::Acquire))
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

fn print_error(ws: &Workspace<'_>, path: &Path, error: FormattingError) {
    match error {
        FormattingError::ParsingError(error) => {
            for entry in error.iter() {
                let msg = entry
                    .message()
                    .strip_suffix('\n')
                    .unwrap_or(entry.message());
                match entry.severity() {
                    Severity::Error => ws.config().ui().error(msg),
                    Severity::Warning => ws.config().ui().warn(msg),
                };
            }
        }
        FormattingError::Error(error) => {
            let error = error.context(format!("cannot format file {}", path.display()));
            ws.config().ui().error(error.to_string());
        }
    }
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
struct TextWithNewline(String);
impl Message for TextWithNewline {
    fn print_text(self)
    where
        Self: Sized,
    {
        print!("{}", self.0);
    }
}

pub enum EmitMode {
    WithPath,
    WithoutPath,
}

pub trait Emittable {
    fn emit(&self, ws: &Workspace<'_>, path: &Path, formatted: &str, emit_mode: EmitMode);
}

impl Emittable for FmtEmitTarget {
    fn emit(&self, ws: &Workspace<'_>, path: &Path, formatted: &str, emit_mode: EmitMode) {
        match (self, emit_mode) {
            (Self::Stdout, EmitMode::WithPath) => ws.config().ui().print(TextWithNewline(format!(
                "{}:\n\n{}",
                path.display(),
                formatted
            ))),
            (Self::Stdout, EmitMode::WithoutPath) => ws
                .config()
                .ui()
                .print(TextWithNewline(formatted.to_string())),
        }
    }
}

fn emit_formatted_file(
    fmt: &CairoFormatter,
    target: &dyn Emittable,
    ws: &Workspace<'_>,
    path: &Path,
    emit_mode: EmitMode,
) -> bool {
    match fmt.format_to_string(&path) {
        Ok(FormatOutcome::Identical(original)) => {
            target.emit(ws, path, &original, emit_mode);
            true
        }
        Ok(FormatOutcome::DiffFound(diff)) => {
            target.emit(ws, path, &diff.formatted, emit_mode);
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

        let success = match &self.opts.action {
            FmtAction::Fix => format_file_in_place(self.fmt, self.opts, self.ws, path),
            FmtAction::Check => check_file_formatting(self.fmt, self.opts, self.ws, path),
            FmtAction::Emit(target) => {
                emit_formatted_file(self.fmt, target, self.ws, path, EmitMode::WithPath)
            }
        };

        if !success {
            self.all_correct.store(false, Ordering::Release);
        }

        Continue
    }
}
