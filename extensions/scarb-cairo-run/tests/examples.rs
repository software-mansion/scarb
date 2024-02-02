use assert_fs::TempDir;
use indoc::indoc;
use snapbox::cmd::{cargo_bin, Command};

use scarb_test_support::cargo::manifest_dir;

#[test]
fn scarb_build_is_called() {
    let example = manifest_dir()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join("hello_world");

    let t = TempDir::new().unwrap();

    Command::new(cargo_bin("scarb"))
        .env("SCARB_TARGET_DIR", t.path())
        .arg("cairo-run")
        .current_dir(example)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
               Compiling hello_world v0.1.0 ([..]/Scarb.toml)
                Finished release target(s) in [..]
                 Running hello_world
            Run completed successfully, returning [987]
        "#});
}

#[test]
fn build_can_be_skipped() {
    let example = manifest_dir()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join("hello_world");

    let t = TempDir::new().unwrap();

    Command::new(cargo_bin("scarb"))
        .env("SCARB_TARGET_DIR", t.path())
        .arg("cairo-run")
        .arg("--no-build")
        .current_dir(example)
        .assert()
        .failure()
        .stderr_eq(indoc! {r#"
            Error: package has not been compiled, file does not exist: hello_world.sierra.json
            help: run `scarb build` to compile the package

        "#});
}

#[test]
fn can_limit_gas() {
    let example = manifest_dir()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join("hello_world");

    let t = TempDir::new().unwrap();

    Command::new(cargo_bin("scarb"))
        .env("SCARB_TARGET_DIR", t.path())
        .arg("cairo-run")
        .arg("--available-gas")
        .arg("100000")
        .current_dir(example)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
               Compiling hello_world v0.1.0 ([..]/Scarb.toml)
                Finished release target(s) in [..]
                 Running hello_world
            Run completed successfully, returning [987]
            Remaining gas: 67840
        "#});
}

#[test]
fn can_disable_gas() {
    let example = manifest_dir()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join("hello_world");

    let t = TempDir::new().unwrap();

    Command::new(cargo_bin("scarb"))
        .env("SCARB_TARGET_DIR", t.path())
        .arg("cairo-run")
        .arg("--available-gas")
        .arg("0")
        .current_dir(example)
        .assert()
        .failure()
        .stderr_eq(indoc! {r#"
            Error: program requires gas counter, please provide `--available-gas` argument
        "#});
}

#[test]
fn use_profiler() {
    let example = manifest_dir()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join("hello_world");

    let t = TempDir::new().unwrap();

    Command::new(cargo_bin("scarb"))
        .env("SCARB_TARGET_DIR", t.path())
        .arg("cairo-run")
        .arg("--run-profiler")
        .current_dir(example)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
               Compiling hello_world v0.1.0 ([..]/Scarb.toml)
                Finished release target(s) in [..] seconds
                 Running hello_world
            Run completed successfully, returning [987]
            Profiling info:
            Weight by sierra statement:
              statement 37: 51 (withdraw_gas_all([0], [1], [5]) { fallthrough([6], [7]) 63([8], [9]) })
              statement 35: 34 (get_builtin_costs() -> ([5]))
              statement 36: 17 (store_temp<BuiltinCosts>([5]) -> ([5]))
              statement 40: 17 (store_temp<RangeCheck>([6]) -> ([6]))
              statement 41: 17 (felt252_is_zero([10]) { fallthrough() 50([11]) })
              statement 56: 16 (store_temp<RangeCheck>([6]) -> ([6]))
              statement 57: 16 (store_temp<GasBuiltin>([7]) -> ([7]))
              statement 58: 16 (store_temp<felt252>([16]) -> ([16]))
              statement 59: 16 (store_temp<felt252>([4]) -> ([4]))
              statement 60: 16 (store_temp<felt252>([18]) -> ([18]))
              statement 61: 16 (function_call<user@hello_world::fib[expr25]>([6], [7], [16], [4], [18]) -> ([19], [20], [21]))
              statement 62: 16 (return([19], [20], [21]))
              statement 48: 5 (store_temp<core::panics::PanicResult::<(core::felt252, core::felt252, core::felt252, core::felt252)>>([14]) -> ([14]))
              statement 26: 3 (store_temp<core::panics::PanicResult::<(core::felt252,)>>([15]) -> ([15]))
              statement 2: 1 (store_temp<RangeCheck>([0]) -> ([0]))
              statement 3: 1 (store_temp<GasBuiltin>([1]) -> ([1]))
              statement 4: 1 (store_temp<felt252>([2]) -> ([2]))
              statement 5: 1 (function_call<user@hello_world::fib>([0], [1], [2]) -> ([3], [4], [5]))
              statement 6: 1 (return([3], [4], [5]))
              statement 10: 1 (store_temp<RangeCheck>([0]) -> ([0]))
              statement 11: 1 (store_temp<GasBuiltin>([1]) -> ([1]))
              statement 12: 1 (store_temp<felt252>([2]) -> ([2]))
              statement 13: 1 (store_temp<felt252>([3]) -> ([3]))
              statement 14: 1 (store_temp<felt252>([4]) -> ([4]))
              statement 15: 1 (function_call<user@hello_world::fib[expr25]>([0], [1], [2], [3], [4]) -> ([5], [6], [7]))
              statement 16: 1 (enum_match<core::panics::PanicResult::<(core::felt252, core::felt252, core::felt252, core::felt252)>>([7]) { fallthrough([8]) 28([9]) })
              statement 24: 1 (store_temp<RangeCheck>([5]) -> ([5]))
              statement 25: 1 (store_temp<GasBuiltin>([6]) -> ([6]))
              statement 27: 1 (return([5], [6], [15]))
              statement 46: 1 (store_temp<RangeCheck>([6]) -> ([6]))
              statement 47: 1 (store_temp<GasBuiltin>([7]) -> ([7]))
              statement 49: 1 (return([6], [7], [14]))
            Weight by concrete libfunc:
              libfunc store_temp<felt252>: 52
              libfunc withdraw_gas_all: 51
              libfunc store_temp<RangeCheck>: 37
              libfunc get_builtin_costs: 34
              libfunc store_temp<GasBuiltin>: 20
              libfunc felt252_is_zero: 17
              libfunc function_call<user@hello_world::fib[expr25]>: 17
              libfunc store_temp<BuiltinCosts>: 17
              libfunc store_temp<core::panics::PanicResult::<(core::felt252, core::felt252, core::felt252, core::felt252)>>: 5
              libfunc store_temp<core::panics::PanicResult::<(core::felt252,)>>: 3
              libfunc enum_match<core::panics::PanicResult::<(core::felt252, core::felt252, core::felt252, core::felt252)>>: 1
              libfunc function_call<user@hello_world::fib>: 1
              return: 19
            Weight by generic libfunc:
              libfunc store_temp: 134
              libfunc withdraw_gas_all: 51
              libfunc get_builtin_costs: 34
              libfunc function_call: 18
              libfunc felt252_is_zero: 17
              libfunc enum_match: 1
              return: 19
            Weight by user function (inc. generated):
              function hello_world::fib[expr25]: 256
              function hello_world::fib: 13
              function hello_world::main: 5
            Weight by original user function:
              function hello_world::fib: 269
              function hello_world::main: 5
            Weight by Cairo function:
              function unknown: 274
            Weight by Sierra stack trace:
              hello_world::main: 274
              hello_world::main -> hello_world::fib: 269
              hello_world::main -> hello_world::fib -> hello_world::fib[expr25]: 256
              hello_world::main -> hello_world::fib -> hello_world::fib[expr25] -> hello_world::fib[expr25]: 241
              hello_world::main -> hello_world::fib -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25]: 226
              hello_world::main -> hello_world::fib -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25]: 211
              hello_world::main -> hello_world::fib -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25]: 196
              hello_world::main -> hello_world::fib -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25]: 181
              hello_world::main -> hello_world::fib -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25]: 166
              hello_world::main -> hello_world::fib -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25]: 151
              hello_world::main -> hello_world::fib -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25]: 136
              hello_world::main -> hello_world::fib -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25]: 121
              hello_world::main -> hello_world::fib -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25]: 106
              hello_world::main -> hello_world::fib -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25]: 91
              hello_world::main -> hello_world::fib -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25]: 76
              hello_world::main -> hello_world::fib -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25]: 61
              hello_world::main -> hello_world::fib -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25]: 46
              hello_world::main -> hello_world::fib -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25]: 31
              hello_world::main -> hello_world::fib -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25] -> hello_world::fib[expr25]: 16

        "#});
}
