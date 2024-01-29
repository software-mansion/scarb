use std::ffi::OsStr;
use std::io::{self, stdout, Write};
use std::iter::once;
use std::path::PathBuf;
use std::process::Command;

use thiserror::Error;

use crate::command::internal_command::InternalScarbCommandBuilder;
use crate::{Metadata, VersionPin};

/// Error thrown while trying to read `scarb metadata`.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum MetadataCommandError {
    /// `scarb metadata` command did not produce any metadata
    #[error("`scarb metadata` command did not produce any metadata")]
    NotFound {
        /// Captured standard output if any.
        stdout: String,
    },

    /// Failed to read `scarb metadata` output.
    #[error("failed to read `scarb metadata` output")]
    Io(#[from] io::Error),

    /// Failed to deserialize `scarb metadata` output.
    #[error("failed to deserialize `scarb metadata` output")]
    Json(#[from] serde_json::Error),

    /// Error during execution of `scarb metadata`.
    #[error("`scarb metadata` exited with error\n\nstdout:\n{stdout}\nstderr:\n{stderr}")]
    ScarbError {
        /// Captured standard output if any.
        stdout: String,
        /// Captured standard error if any.
        stderr: String,
    },
}

impl MetadataCommandError {
    /// Check if this is [`MetadataCommandError::NotFound`].
    pub const fn did_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. })
    }
}

/// A builder for `scarb metadata` command invocation.
///
/// In detail, this will always execute `scarb --json metadata --format-version N`, where `N`
/// matches metadata version understandable by this crate version.
#[derive(Clone, Debug, Default)]
pub struct MetadataCommand {
    inner: InternalScarbCommandBuilder,
    no_deps: bool,
    inherit_stdout: bool,
}

impl MetadataCommand {
    /// Creates a default `scarb metadata` command, which will look for `scarb` in `$PATH` and
    /// for `Scarb.toml` in the current directory or its ancestors.
    pub fn new() -> Self {
        Self::default()
    }

    /// Path to `scarb` executable.
    ///
    /// If not set, this will use the `$SCARB` environment variable, and if that is not set, it
    /// will simply be `scarb` and the system will look it up in `$PATH`.
    pub fn scarb_path(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.inner.scarb_path(path);
        self
    }

    /// Path to `Scarb.toml`.
    ///
    /// If not set, this will look for `Scarb.toml` in the current directory or its ancestors.
    pub fn manifest_path(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.inner.manifest_path(path);
        self
    }

    /// Current directory of the `scarb metadata` process.
    pub fn current_dir(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.inner.current_dir(path);
        self
    }

    /// Output information only about workspace members and don't fetch dependencies.
    pub fn no_deps(&mut self) -> &mut Self {
        self.no_deps = true;
        self
    }

    /// Inserts or updates an environment variable mapping.
    pub fn env(&mut self, key: impl AsRef<OsStr>, val: impl AsRef<OsStr>) -> &mut Self {
        self.inner.env(key, val);
        self
    }

    /// Adds or updates multiple environment variable mappings.
    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.inner.envs(vars);
        self
    }

    /// Removes an environment variable mapping.
    pub fn env_remove(&mut self, key: impl AsRef<OsStr>) -> &mut Self {
        self.inner.env_remove(key);
        self
    }

    /// Clears the entire environment map for the child process.
    pub fn env_clear(&mut self) -> &mut Self {
        self.inner.env_clear();
        self
    }

    /// Inherit standard error, i.e. show Scarb errors in this process's standard error.
    pub fn inherit_stderr(&mut self) -> &mut Self {
        self.inner.inherit_stderr();
        self
    }

    /// Inherit standard output, i.e. show Scarb output in this process's standard output.
    pub fn inherit_stdout(&mut self) -> &mut Self {
        // we can not just use self.inner.inherit_stdout()
        // because it will make output.stdout empty
        self.inherit_stdout = true;
        self
    }

    fn scarb_command(&self) -> Command {
        let mut builder = self.inner.clone();
        builder.args(["metadata", "--format-version"]);
        builder.arg(VersionPin.numeric().to_string());
        if self.no_deps {
            builder.arg("--no-deps");
        }
        builder.command()
    }

    /// Runs configured `scarb metadata` and returns parsed `Metadata`.
    pub fn exec(&self) -> Result<Metadata, MetadataCommandError> {
        let mut cmd = self.scarb_command();
        let output = cmd.output()?;
        if !output.status.success() {
            if self.inherit_stdout {
                stdout().write_all(&output.stdout)?;
            }
            return Err(MetadataCommandError::ScarbError {
                stdout: String::from_utf8_lossy(&output.stdout).into(),
                stderr: String::from_utf8_lossy(&output.stderr).into(),
            });
        }
        parse_stream(output.stdout.as_slice())
    }
}

fn parse_stream(data: &[u8]) -> Result<Metadata, MetadataCommandError> {
    let mut err = None;

    let data = std::str::from_utf8(data).unwrap();

    let mut lines = data.split("\n").map(|line| line.trim_end());

    macro_rules! json_parse {
        ($json:expr) => {
            match serde_json::from_str($json) {
                Ok(metadata) => return Ok(metadata),
                Err(serde_err) => {
                    err = Some(
                        if serde_err.is_data()
                            && !serde_err.to_string().contains("expected metadata version")
                        {
                            MetadataCommandError::NotFound {
                                stdout: data.into(),
                            }
                        } else {
                            serde_err.into()
                        },
                    )
                }
            }
        };
    }

    const OPEN_BRACKET: &'static str = "{";
    const CLOSE_BRACKET: &'static str = "}";

    // depending on usage of --json flag scarb returns either one line json
    // or pretty printed one which starts with "{" and ends with "}" on single lines
    //
    // singleline json's -- it should be useless since we do not use --json flag
    // but better safe than sorry
    for line in lines
        .clone()
        .filter(|line| line.starts_with(OPEN_BRACKET) && line.ends_with(CLOSE_BRACKET))
    {
        json_parse!(line);
    }
    // multiline json's
    loop {
        let json_lines = lines
            .by_ref()
            .skip_while(|line| *line != OPEN_BRACKET)
            .skip(1)
            .take_while(|line| *line != CLOSE_BRACKET);

        let json_string = once(OPEN_BRACKET)
            .chain(json_lines)
            .chain(once(CLOSE_BRACKET))
            .collect::<Vec<&str>>()
            .join("");

        if json_string == format!("{OPEN_BRACKET}{CLOSE_BRACKET}") {
            break;
        }

        json_parse!(&json_string);
    }

    Err(err.unwrap_or_else(|| MetadataCommandError::NotFound {
        stdout: data.into(),
    }))
}

#[cfg(test)]
mod tests {
    use semver::Version;

    use crate::{CairoVersionInfo, Metadata, MetadataCommandError, VersionInfo, WorkspaceMetadata};

    macro_rules! check_parse_stream {
        ($input:expr, $expected:pat) => {{
            #![allow(clippy::redundant_pattern_matching)]
            let actual = crate::command::metadata_command::parse_stream(
                $input
                    .to_string()
                    .replace("{meta}", &minimal_metadata_json())
                    .as_bytes(),
            );

            assert!(matches!(actual, $expected));
        }};
    }

    #[test]
    fn parse_stream_ok() {
        check_parse_stream!("{meta}", Ok(_));
    }

    #[test]
    fn parse_stream_ok_nl() {
        check_parse_stream!("{meta}\n", Ok(_));
    }

    #[test]
    fn parse_stream_trailing_nl() {
        check_parse_stream!("\n\n\n\n{meta}\n\n\n", Ok(_));
    }

    #[test]
    fn parse_stream_ok_random_text_around() {
        check_parse_stream!("abcde\n{meta}\nghjkl", Ok(_));
    }

    #[test]
    fn parse_stream_empty() {
        check_parse_stream!("", Err(MetadataCommandError::NotFound { .. }));
    }

    #[test]
    fn parse_stream_empty_nl() {
        check_parse_stream!("\n", Err(MetadataCommandError::NotFound { .. }));
    }

    #[test]
    fn parse_stream_garbage_message() {
        check_parse_stream!("{\"foo\":1}", Err(MetadataCommandError::NotFound { .. }));
    }

    #[test]
    fn parse_stream_garbage_message_nl() {
        check_parse_stream!("{\"foo\":1}\n", Err(MetadataCommandError::NotFound { .. }));
    }

    #[test]
    fn parse_stream_garbage_messages() {
        check_parse_stream!(
            "{\"foo\":1}\n{\"bar\":1}",
            Err(MetadataCommandError::NotFound { .. })
        );
    }

    #[test]
    fn parse_stream_not_serializable() {
        check_parse_stream!(
            "{\"version\":\"x\",\"foo\":1}",
            Err(MetadataCommandError::Json(_))
        );
    }

    #[test]
    fn parse_stream_version_0() {
        check_parse_stream!(
            "{\"version\":0,\"foo\":1}",
            Err(MetadataCommandError::Json(_))
        );
    }

    #[test]
    fn parse_stream_impersonator() {
        check_parse_stream!("{\"version\":0,\"foo\":1}\n{meta}", Ok(_));
    }

    #[test]
    fn parse_stream_crlf() {
        check_parse_stream!(
            "{\"foo\":1}\r\n{\"foo\":1}\r\n{meta}\r\n{\"foo\":1}\r\n",
            Ok(_)
        );
    }

    fn minimal_metadata_json() -> String {
        serde_json::to_string(&minimal_metadata()).unwrap()
    }

    fn minimal_metadata() -> Metadata {
        Metadata {
            version: Default::default(),
            app_exe: Default::default(),
            app_version_info: VersionInfo {
                version: Version::new(1, 0, 0),
                commit_info: Default::default(),
                cairo: CairoVersionInfo {
                    version: Version::new(1, 0, 0),
                    commit_info: Default::default(),
                    extra: Default::default(),
                },
                extra: Default::default(),
            },
            target_dir: Default::default(),
            runtime_manifest: Default::default(),
            workspace: WorkspaceMetadata {
                manifest_path: Default::default(),
                root: Default::default(),
                members: Default::default(),
                extra: Default::default(),
            },
            packages: Default::default(),
            compilation_units: Default::default(),
            current_profile: "dev".into(),
            profiles: vec!["dev".into()],
            extra: Default::default(),
        }
    }
}
