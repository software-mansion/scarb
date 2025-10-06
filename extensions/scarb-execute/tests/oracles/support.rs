use assert_fs::TempDir;
use camino::Utf8PathBuf;
use derive_builder::Builder;
use indoc::{formatdoc, indoc};
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct Check {
    #[builder(setter(into))]
    lib_cairo: String,

    #[builder(default, setter(custom))]
    failure: bool,
    #[builder(setter(into))]
    stdout_matches: String,
    #[builder(default, setter(into))]
    stderr_contains: String,

    #[builder(default = "true")]
    enable_experimental_oracles_flag: bool,

    #[builder(default, setter(custom))]
    profile: Option<String>,

    #[builder(default, setter(custom))]
    pb_ops: Vec<Box<dyn FnOnce(ProjectBuilder) -> ProjectBuilder>>,

    #[builder(default, setter(custom))]
    #[expect(clippy::type_complexity)] // this is not complex at all
    dir_ops: Vec<Box<dyn FnOnce(&TempDir)>>,
}

impl CheckBuilder {
    pub fn check(self) -> TempDir {
        self.build().unwrap().check()
    }

    pub fn failure(mut self) -> Self {
        self.failure = Some(true);
        self
    }

    pub fn profile(mut self, profile: String) -> Self {
        self.profile = Some(Some(profile));
        self
    }

    pub fn pb_op(mut self, op: impl FnOnce(ProjectBuilder) -> ProjectBuilder + 'static) -> Self {
        self.pb_ops.get_or_insert_default().push(Box::new(op));
        self
    }

    pub fn dir_op(mut self, op: impl FnOnce(&TempDir) + 'static) -> Self {
        self.dir_ops.get_or_insert_default().push(Box::new(op));
        self
    }

    pub fn asset(self, path: impl Into<Utf8PathBuf>, content: impl Into<Vec<u8>>) -> Self {
        let path = path.into();
        let content = content.into();
        self.pb_op(move |t| {
            t.src_binary(&path, content)
                .manifest_package_extra(formatdoc! {r#"
                    assets = [{path:?}]
                "#})
        })
    }
}

impl Check {
    pub fn check(self) -> TempDir {
        let t = TempDir::new().unwrap();
        let mut pb = ProjectBuilder::start()
            .name("oracle_test")
            .version("0.1.0")
            .manifest_extra(indoc! {r#"
                [executable]
                
                [cairo]
                enable-gas = false
            "#})
            .dep_cairo_execute()
            // NOTE: We use this just to access `cheatcode` libfunc.
            .dep_starknet()
            .dep_oracle_asserts()
            .lib_cairo(self.lib_cairo);

        for op in self.pb_ops {
            pb = op(pb);
        }

        pb.build(&t);

        for op in self.dir_ops {
            op(&t);
        }

        let mut snapbox = Scarb::quick_snapbox().env("RUST_BACKTRACE", "0");

        if let Some(profile) = &self.profile {
            snapbox = snapbox.args(vec!["--profile", profile]);
        }

        snapbox = snapbox.arg("execute").current_dir(&t);

        if self.enable_experimental_oracles_flag {
            snapbox = snapbox.arg("--experimental-oracles");
        }

        let mut assert = snapbox.assert();

        if self.failure {
            assert = assert.failure();
        } else {
            assert = assert.success();
        }

        assert = assert.stdout_matches(self.stdout_matches);

        if !self.stderr_contains.is_empty() {
            let pattern = self.stderr_contains;
            let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
            assert!(
                stderr.contains(&pattern),
                "stderr does not contain: {pattern:?}\n\nstderr:\n{stderr}"
            );
        }

        t
    }
}
