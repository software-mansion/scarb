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

    #[builder(default, setter(custom))]
    profile: Option<String>,
}

impl CheckBuilder {
    fn check(&mut self) {
        self.build().unwrap().check();
    }

    fn failure(&mut self) -> &mut Self {
        self.failure = Some(true);
        self
    }

    fn profile(&mut self, profile: String) -> &mut Self {
        self.profile = Some(Some(profile));
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
            // NOTE: We use this just to access `cheatcode` libfunc.
            .dep_starknet()
            .dep_oracle_asserts()
            .lib_cairo(self.lib_cairo)
            .cp_test_oracle("test_oracle.py")
            .build(&t);

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
        .profile("release".to_string())
        .failure()
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `release` profile target(s) in [..]
            [..]Executing oracle_test
            error: Cairo program run failed: Error at pc=0:33:
            Got an exception while executing a hint: Oracles are experimental feature. To enable, pass --experimental-oracles CLI flag.
            Cairo traceback (most recent call last):
            Unknown location (pc=0:2)
            Unknown location (pc=0:10)

        "#})
        .check();
}

/// This tests checks two things at once:
/// 1. That we react to binaries that don't talk JSON-RPC.
/// 2. That we look for binaries in $PATH.
#[test]
fn oracle_invoke_non_jsonrpc_command_from_path() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "stdio:whoami";
                connection_string.serialize(ref inputs);
                'pow'.serialize(ref inputs);
                (4).serialize(ref inputs);
                (2).serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<Span<felt252>>(result);
            }
        "#})
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Err("oracle process is misbehaving: expected JSON-RPC message starting with '{', got byte: '[..]'")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}

#[test]
fn oracle_invoke_unknown_scheme() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "unknown:///test";
                connection_string.serialize(ref inputs);
                'foo'.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<ByteArray>(result);
            }
        "#})
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Err("unsupported connection scheme: "unknown:///test"
            note: supported schemes are: "stdio"")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}

#[test]
fn oracle_invoke_missing_connection_scheme() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "no scheme";
                connection_string.serialize(ref inputs);
                'foo'.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<ByteArray>(result);
            }
        "#})
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Err("unsupported connection scheme: "no scheme"
            note: supported schemes are: "stdio"")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}

#[test]
fn oracle_invoke_non_existent_file() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "stdio:i_definitelly_do_not_exist.exe";
                connection_string.serialize(ref inputs);
                'hello'.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<Span<felt252>>(result);
            }
        "#})
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Err("failed to spawn oracle process

            Caused by:
                [..]")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}

#[test]
fn oracle_invoke_non_executable_file() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "stdio:Scarb.toml";
                connection_string.serialize(ref inputs);
                'hello'.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<Span<felt252>>(result);
            }
        "#})
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Err("failed to spawn oracle process

            Caused by:
                [..]")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}

/// If the `./ ` prefix is omitted from the path, then the oracle should be looked for in $PATH,
/// which doesn't have `.` in it.
#[test]
fn oracle_invoke_test_oracle_without_dot_slash() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "stdio:test_oracle.py";
                connection_string.serialize(ref inputs);
                'panic'.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<Span<felt252>>(result);
            }
        "#})
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Err("failed to spawn oracle process

            Caused by:
                [..]")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}

#[test]
fn oracle_invoke_test_oracle() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let connection_string: ByteArray = "stdio:python3 ./test_oracle.py";

                let mut inputs: Array<felt252> = array![];
                connection_string.serialize(ref inputs);
                'sqrt'.serialize(ref inputs);
                (16).serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<felt252>(result);

                let mut inputs: Array<felt252> = array![];
                connection_string.serialize(ref inputs);
                'panic'.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<felt252>(result);
            }
        "#})
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Ok(4)
            Result::Err("oops")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}
