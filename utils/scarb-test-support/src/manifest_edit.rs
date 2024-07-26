use std::cell::LazyCell;
use std::ffi::OsStr;
use std::fs;

use assert_fs::fixture::ChildPath;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use snapbox::cmd::Command;

use crate::command::Scarb;

pub struct ManifestEditHarness {
    cmd: Command,
    path: Option<ChildPath>,
    input_manifest: Option<String>,
    output_manifest: Option<String>,
    should_fail: bool,
    stdout_matches: String,
}

impl ManifestEditHarness {
    pub fn new() -> Self {
        Self {
            cmd: Scarb::quick_snapbox(),
            path: None,
            input_manifest: None,
            output_manifest: None,
            should_fail: false,
            stdout_matches: "".to_string(),
        }
    }

    pub fn offline() -> Self {
        Self::new().arg("--offline")
    }

    pub fn run(self) {
        let t = LazyCell::new(|| TempDir::new().unwrap());
        let t = self.path.unwrap_or_else(|| t.child("proj"));

        let input_manifest = self.input_manifest.unwrap();
        t.child("Scarb.toml").write_str(&input_manifest).unwrap();

        t.child("src/lib.cairo")
            .write_str("fn foo() -> felt252 { 42 }")
            .unwrap();

        let cmd = self.cmd.current_dir(&t).assert();

        let cmd = if self.should_fail {
            cmd.failure()
        } else {
            cmd.success()
        };

        cmd.stdout_matches(self.stdout_matches);

        assert_eq!(
            fs::read_to_string(t.child("Scarb.toml").path()).unwrap(),
            self.output_manifest.unwrap_or(input_manifest)
        );
    }

    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.cmd = self.cmd.arg(arg);
        self
    }

    pub fn args(mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> Self {
        self.cmd = self.cmd.args(args);
        self
    }

    pub fn path(mut self, t: ChildPath) -> Self {
        self.path = Some(t);
        self
    }

    pub fn input(mut self, input_manifest: impl ToString) -> Self {
        self.input_manifest = Some(input_manifest.to_string());
        self
    }

    pub fn output(mut self, output_manifest: impl ToString) -> Self {
        self.output_manifest = Some(output_manifest.to_string());
        self
    }

    pub fn failure(mut self) -> Self {
        self.should_fail = true;
        self
    }

    pub fn stdout_matches(mut self, stdout_matches: impl ToString) -> Self {
        self.stdout_matches = stdout_matches.to_string();
        self
    }
}

impl Default for ManifestEditHarness {
    fn default() -> Self {
        Self::new()
    }
}
