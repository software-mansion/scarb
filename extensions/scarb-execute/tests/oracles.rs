use assert_fs::TempDir;
use assert_fs::prelude::*;
use derive_builder::Builder;
use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::fsx::make_executable;
use scarb_test_support::project_builder::ProjectBuilder;
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

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
            // NOTE: We use this just to access `cheatcode` libfunc.
            .dep_starknet()
            .dep(
                "oracle_asserts",
                Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/oracle_asserts"),
            )
            .lib_cairo(self.lib_cairo)
            .cp("tests/test_oracle.py", "test_oracle.py")
            .build(&t);

        make_executable(t.child("test_oracle.py").path());

        let mut snapbox = Scarb::quick_snapbox()
            .env("RUST_BACKTRACE", "0")
            .arg("execute")
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
fn oracle_invoke_non_jsonrpc_command() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "stdio:/usr/bin/yes";
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
            Result::Err("oracle process is misbehaving: expected JSON-RPC message starting with '{', got byte: 'y'")
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
            Result::Err("unsupported connection scheme: unknown:///test
            note: supported schemes are: `stdio`")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}

#[test]
fn oracle_invoke_invalid_url() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "not a url";
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
            Result::Err("relative URL without a base")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}

/// Smoke tests that `test_oracle.py` actually works as intended.
#[test]
#[cfg_attr(
    not(target_family = "unix"),
    ignore = "This test relies on UNIX shebangs."
)]
fn oracle_json_rpc_smoke_test() {
    // Spawn test_oracle.py process and grab it's I/O.
    let mut process = Command::new("tests/test_oracle.py")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();

    let mut stdin = process.stdin.take().unwrap();
    let stdout = process.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    let mut send = |msg: &str| {
        writeln!(stdin, "{msg}").unwrap();
    };

    let mut recv = |expected_msg: &str| {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        line = line.trim_end().to_string();
        assert_eq!(line, expected_msg);
    };

    // Communication sequence that we expect to function properly.
    recv(r#"{"jsonrpc": "2.0", "id": 0, "method": "ready"}"#);
    send(r#"{"jsonrpc": "2.0", "id": 0, "result": {}}"#);

    send(
        r#"{"jsonrpc": "2.0", "id": 0, "method": "invoke", "params": {"selector": "sqrt", "calldata": ["0x10"]}}"#,
    );
    recv(r#"{"jsonrpc": "2.0", "id": 0, "result": ["0x4"]}"#);

    send(
        r#"{"jsonrpc": "2.0", "id": 1, "method": "invoke", "params": {"selector": "panic", "calldata": []}}"#,
    );
    recv(r#"{"jsonrpc": "2.0", "id": 1, "error": {"code": 0, "message": "oops"}}"#);

    send(r#"{"jsonrpc": "2.0", "method": "shutdown"}"#);

    // Close stdin to the signal end of the input.
    drop(stdin);

    // Wait for a process to terminate.
    let status = process.wait().unwrap();
    assert!(status.success(), "oracle process should exit successfully");
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
            Result::Err("failed to spawn oracle process")
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
            Result::Err("failed to spawn oracle process")
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
            Result::Err("failed to spawn oracle process")
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
                let connection_string: ByteArray = "stdio:./test_oracle.py";

                let mut inputs: Array<felt252> = array![];
                connection_string.serialize(ref inputs);
                'sqrt'.serialize(ref inputs);
                (16).serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<Span<felt252>>(result);

                let mut inputs: Array<felt252> = array![];
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
            Result::Ok([4])
            Result::Err("oops")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}
