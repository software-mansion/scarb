use assert_fs::prelude::*;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

const EXPERIMENTAL_LIBFUNC: &str = indoc! {r#"
    extern fn redeposit_gas() implicits(GasBuiltin) nopanic;

    #[starknet::contract]
    mod ExperimentalLibfunc {
        #[storage]
        struct Storage {}

        #[external(v0)]
        fn experiment(self: @ContractState) {
            super::redeposit_gas();
        }
    }
"#};

const TESTING_LIST: &str = indoc! {r#"
    {
        "allowed_libfuncs": []
    }
"#};

#[test]
fn default_behaviour() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
        "#})
        .dep_starknet()
        .lib_cairo(EXPERIMENTAL_LIBFUNC)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        warn: libfunc `redeposit_gas` is not allowed in the libfuncs list `Default libfunc list`
         --> contract: ExperimentalLibfunc
        help: try compiling with the `experimental` list
         --> Scarb.toml
            [[target.starknet-contract]]
            allowed-libfuncs-list.name = "experimental"

        [..]  Finished release target(s) in [..]
        "#});
}

#[test]
fn check_true() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            allowed-libfuncs = true
        "#})
        .dep_starknet()
        .lib_cairo(EXPERIMENTAL_LIBFUNC)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        warn: libfunc `redeposit_gas` is not allowed in the libfuncs list `Default libfunc list`
         --> contract: ExperimentalLibfunc
        help: try compiling with the `experimental` list
         --> Scarb.toml
            [[target.starknet-contract]]
            allowed-libfuncs-list.name = "experimental"

        [..]  Finished release target(s) in [..]
        "#});
}

#[test]
fn check_false() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            allowed-libfuncs = false
        "#})
        .dep_starknet()
        .lib_cairo(EXPERIMENTAL_LIBFUNC)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        [..]  Finished release target(s) in [..]
        "#});
}

#[test]
fn deny_true() {
    let t = assert_fs::TempDir::new().unwrap();

    t.child("testing_list.json")
        .write_str(TESTING_LIST)
        .unwrap();

    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            allowed-libfuncs-deny = true
            allowed-libfuncs-list.path = "testing_list.json"
        "#})
        .dep_starknet()
        .lib_cairo(EXPERIMENTAL_LIBFUNC)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        error: libfunc `revoke_ap_tracking` is not allowed in the libfuncs list `[..]testing_list.json`
         --> contract: ExperimentalLibfunc

        error: aborting compilation, because contracts use disallowed Sierra libfuncs
        error: could not compile `hello` due to previous error
        "#});
}

#[test]
fn pass_named_list() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            allowed-libfuncs-list.name = "experimental"
        "#})
        .dep_starknet()
        .lib_cairo(EXPERIMENTAL_LIBFUNC)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        [..]  Finished release target(s) in [..]
        "#});
}

#[test]
fn unknown_list_name() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            allowed-libfuncs-list.name = "definitely does not exist"
        "#})
        .dep_starknet()
        .lib_cairo(EXPERIMENTAL_LIBFUNC)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        error: failed to check allowed libfuncs for contract: ExperimentalLibfunc

        Caused by:
            No libfunc list named 'definitely does not exist' is known.
        error: could not compile `hello` due to previous error
        "#});
}

#[test]
fn list_path() {
    let t = assert_fs::TempDir::new().unwrap();

    t.child("testing_list.json")
        .write_str(TESTING_LIST)
        .unwrap();

    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            allowed-libfuncs-list.path = "testing_list.json"
        "#})
        .dep_starknet()
        .lib_cairo(EXPERIMENTAL_LIBFUNC)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        warn: libfunc `revoke_ap_tracking` is not allowed in the libfuncs list `[..]testing_list.json`
         --> contract: ExperimentalLibfunc

        [..]  Finished release target(s) in [..]
        "#});
}

#[test]
fn list_path_does_not_exist() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            allowed-libfuncs-list.path = "does_not_exist.json"
        "#})
        .dep_starknet()
        .lib_cairo(EXPERIMENTAL_LIBFUNC)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        error: failed to get absolute path of `[..]does_not_exist.json`

        Caused by:
            [..]
        error: could not compile `hello` due to previous error
        "#});
}
