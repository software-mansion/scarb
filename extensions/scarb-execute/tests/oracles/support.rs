use assert_fs::TempDir;
use camino::Utf8PathBuf;
use derive_builder::Builder;
use indoc::indoc;
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

    #[builder(default = "true")]
    enable_experimental_oracles_flag: bool,

    #[builder(default, setter(custom))]
    profile: Option<String>,

    #[builder(default, setter(custom))]
    pb_ops: Vec<Box<dyn FnOnce(ProjectBuilder) -> ProjectBuilder>>,
}

impl CheckBuilder {
    pub fn check(self) {
        self.build().unwrap().check();
    }

    pub fn failure(mut self) -> Self {
        self.failure = Some(true);
        self
    }

    pub fn profile(mut self, profile: String) -> Self {
        self.profile = Some(Some(profile));
        self
    }

    pub fn src_binary(mut self, path: impl Into<Utf8PathBuf>, content: impl Into<Vec<u8>>) -> Self {
        let path = path.into();
        let content = content.into();
        self.pb_ops
            .get_or_insert_default()
            .push(Box::new(move |t| t.src_binary(path, content)));
        self
    }
}

impl Check {
    pub fn check(self) {
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
            .lib_cairo(self.lib_cairo)
            .cp_test_oracle("test_oracle.py");

        for op in self.pb_ops {
            pb = op(pb);
        }

        pb.build(&t);

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

        assert.stdout_matches(self.stdout_matches);
    }
}
