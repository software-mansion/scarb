use assert_fs::TempDir;
use derive_builder::Builder;
use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[derive(Builder)]
struct Check {
    lib_cairo: &'static str,

    #[builder(default, setter(custom))]
    failure: bool,
    stdout_matches: &'static str,

    #[builder(default = "true")]
    enable_experimental_oracles_flag: bool,
}

impl CheckBuilder {
    fn check(&mut self) {
        self.build().unwrap().check();
    }

    fn failure(&mut self) -> &mut Self {
        self.failure = Some(true);
        self
    }
}

impl Check {
    fn check(self) {
        let t = TempDir::new().unwrap();

        ProjectBuilder::start()
            .name("oracle_test")
            .version("0.1.0")
            .manifest_extra(indoc! {r#"
                [executable]
                
                [cairo]
                enable-gas = false
            "#})
            .dep_cairo_execute()
            // TODO(mkaput): Remove starknet dependency in favour of the oracle package.
            .dep_starknet()
            .lib_cairo(self.lib_cairo)
            .build(&t);

        let mut snapbox = Scarb::quick_snapbox()
            .arg("execute")
            .arg("--print-program-output")
            .current_dir(&t);

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

#[test]
fn oracle_invoke_without_experimental_flag_fails() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                starknet::testing::cheatcode::<'oracle_invoke'>(array![].span());
            }
        "#})
        .enable_experimental_oracles_flag(false)
        .failure()
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            error: Cairo program run failed: Error at pc=0:33:
            Got an exception while executing a hint: Oracles are experimental feature. To enable, pass --experimental-oracles CLI flag.
            Cairo traceback (most recent call last):
            Unknown location (pc=0:2)
            Unknown location (pc=0:10)

        "#})
        .check();
}

#[test]
fn oracle_invoke_direct() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> Span<felt252> {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "connection string";
                connection_string.serialize(ref inputs);
                'pow'.serialize(ref inputs);
                (4).serialize(ref inputs);
                (2).serialize(ref inputs);
                starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span())
            }
        "#})
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Program output:
            3
            0
            1
            9876543210
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}
