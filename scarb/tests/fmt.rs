use std::fs;

use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::fsx;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::workspace_builder::WorkspaceBuilder;

const SIMPLE_ORIGINAL: &str = r"fn main()    ->    felt252      {      42      }";
const SIMPLE_FORMATTED: &str = indoc! {r#"
    fn main() -> felt252 {
        42
    }
    "#
};

fn build_temp_dir(data: &str) -> TempDir {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();
    t.child("src/lib.cairo").write_str(data).unwrap();

    t
}

#[test]
fn simple_check_invalid() {
    let t = build_temp_dir(SIMPLE_ORIGINAL);
    Scarb::quick_snapbox()
        .arg("fmt")
        .arg("--check")
        .arg("--no-color")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {"\
            Diff in [..]/src/lib.cairo:
             --- original
            +++ modified
            @@ -1 +1,3 @@
            -fn main()    ->    felt252      {      42      }
            / No newline at end of file
            +fn main() -> felt252 {
            +    42
            +}

            "});
    let content = fs::read_to_string(t.child("src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_ORIGINAL);
}

#[test]
fn simple_emit_invalid() {
    let t = build_temp_dir(SIMPLE_ORIGINAL);
    Scarb::quick_snapbox()
        .arg("fmt")
        .arg("--emit")
        .arg("stdout")
        .arg("--no-color")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(format!(
            "{}:\n{}\n",
            fsx::canonicalize(t.child("src/lib.cairo"))
                .unwrap()
                .display(),
            SIMPLE_FORMATTED
        ));
    let content = fs::read_to_string(t.child("src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_ORIGINAL);
}

#[test]
fn simple_emit_valid() {
    let t = build_temp_dir(SIMPLE_FORMATTED);
    Scarb::quick_snapbox()
        .arg("fmt")
        .arg("--emit")
        .arg("stdout")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn simple_check_valid() {
    let t = build_temp_dir(SIMPLE_FORMATTED);
    Scarb::quick_snapbox()
        .arg("fmt")
        .arg("--check")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn simple_format() {
    let t = build_temp_dir(SIMPLE_ORIGINAL);
    Scarb::quick_snapbox()
        .arg("fmt")
        .current_dir(&t)
        .assert()
        .success();

    assert!(t.child("src/lib.cairo").is_file());
    let content = fs::read_to_string(t.child("src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_FORMATTED);
}

#[test]
fn simple_format_with_filter() {
    let t = build_temp_dir(SIMPLE_ORIGINAL);
    Scarb::quick_snapbox()
        .args(["fmt", "--package", "world"])
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq("error: package `world` not found in workspace\n");

    assert!(t.child("src/lib.cairo").is_file());
    let content = fs::read_to_string(t.child("src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_ORIGINAL);

    Scarb::quick_snapbox()
        .args(["fmt", "--package", "hell*"])
        .current_dir(&t)
        .assert()
        .success();

    assert!(t.child("src/lib.cairo").is_file());
    let content = fs::read_to_string(t.child("src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_FORMATTED);
}

#[test]
fn format_with_import_sorting() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            [tool.fmt]
            sort-module-level-items = true
            "#,
        )
        .unwrap();
    t.child("src/lib.cairo")
        .write_str(indoc! {"\
            use openzeppelin::introspection::interface;
            use openzeppelin::introspection::first;
            
            #[starknet::contract]
            mod SRC5 {
                use openzeppelin::introspection::interface;
                use openzeppelin::introspection::{interface, AB};
            
                #[storage]
                struct Storage {
                    supported_interfaces: LegacyMap<felt252, bool>
                }
            
                use openzeppelin::introspection::first;
            
                mod A {}
                mod G;
                mod F;
            
                #[external(v0)]
                impl SRC5Impl of interface::ISRC5<ContractState> {
                    fn supports_interface(self: @ContractState, interface_id: felt252) -> bool {
                        true
                    }
                }
            
                use A;
                use starknet::ArrayTrait;
            
                mod Inner {
                    use C;
                    use B;
                }
            }
            "
        })
        .unwrap();

    Scarb::quick_snapbox()
        .args(["fmt", "--check", "--no-color"])
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {"\
        Diff in [..]/src/lib.cairo:
        --- original
       +++ modified
       @@ -1,10 +1,17 @@
       +use openzeppelin::introspection::first;
        use openzeppelin::introspection::interface;
       -use openzeppelin::introspection::first;
       
        #[starknet::contract]
        mod SRC5 {
       +    mod F;
       +    mod G;
       +
       +    use A;
       +
       +    use openzeppelin::introspection::first;
            use openzeppelin::introspection::interface;
            use openzeppelin::introspection::{interface, AB};
       +    use starknet::ArrayTrait;
       
            #[storage]
            struct Storage {
       @@ -11,11 +18,7 @@
                supported_interfaces: LegacyMap<felt252, bool>
            }
       
       -    use openzeppelin::introspection::first;
       -
            mod A {}
       -    mod G;
       -    mod F;
       
            #[external(v0)]
            impl SRC5Impl of interface::ISRC5<ContractState> {
       @@ -24,11 +27,8 @@
                }
            }
       
       -    use A;
       -    use starknet::ArrayTrait;
       -
            mod Inner {
       +        use B;
                use C;
       -        use B;
            }
        }
       
       "});
}

#[test]
fn workspace_with_root() {
    let t = TempDir::new().unwrap().child("test_workspace");
    let pkg1 = t.child("first");
    ProjectBuilder::start()
        .name("first")
        .lib_cairo(SIMPLE_ORIGINAL)
        .build(&pkg1);
    let pkg2 = t.child("second");
    ProjectBuilder::start()
        .name("second")
        .lib_cairo(SIMPLE_ORIGINAL)
        .dep("first", &pkg1)
        .build(&pkg2);
    let root = ProjectBuilder::start()
        .name("some_root")
        .lib_cairo(SIMPLE_ORIGINAL)
        .dep("first", &pkg1)
        .dep("second", &pkg2);
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .package(root)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fmt")
        .current_dir(&t)
        .assert()
        .success();

    let content = fs::read_to_string(t.child("src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_FORMATTED);
    let content = fs::read_to_string(t.child("first/src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_ORIGINAL);
    let content = fs::read_to_string(t.child("second/src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_ORIGINAL);

    Scarb::quick_snapbox()
        .args(["fmt", "--workspace"])
        .current_dir(&t)
        .assert()
        .success();

    let content = fs::read_to_string(t.child("src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_FORMATTED);
    let content = fs::read_to_string(t.child("first/src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_FORMATTED);
    let content = fs::read_to_string(t.child("second/src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_FORMATTED);
}

#[test]
fn workspace_emit_with_root() {
    let t = TempDir::new().unwrap().child("test_workspace");
    let pkg1 = t.child("first");
    ProjectBuilder::start()
        .name("first")
        .lib_cairo(SIMPLE_ORIGINAL)
        .build(&pkg1);
    let pkg2 = t.child("second");
    ProjectBuilder::start()
        .name("second")
        .lib_cairo(SIMPLE_ORIGINAL)
        .dep("first", &pkg1)
        .build(&pkg2);
    let root = ProjectBuilder::start()
        .name("some_root")
        .lib_cairo(SIMPLE_ORIGINAL)
        .dep("first", &pkg1)
        .dep("second", &pkg2);
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .package(root)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fmt")
        .arg("--emit")
        .arg("stdout")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(format!(
            "{}:\n{}\n",
            fsx::canonicalize(t.child("src/lib.cairo"))
                .unwrap()
                .display(),
            SIMPLE_FORMATTED
        ));

    let content = fs::read_to_string(t.child("src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_ORIGINAL);
    let content = fs::read_to_string(t.child("first/src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_ORIGINAL);
    let content = fs::read_to_string(t.child("second/src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_ORIGINAL);

    Scarb::quick_snapbox()
        .args(["fmt", "--workspace", "--emit", "stdout"])
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(format!(
            "{}:\n{}\n{}:\n{}\n{}:\n{}\n",
            fsx::canonicalize(t.child("first/src/lib.cairo"))
                .unwrap()
                .display(),
            SIMPLE_FORMATTED,
            fsx::canonicalize(t.child("second/src/lib.cairo"))
                .unwrap()
                .display(),
            SIMPLE_FORMATTED,
            fsx::canonicalize(t.child("src/lib.cairo"))
                .unwrap()
                .display(),
            SIMPLE_FORMATTED,
        ));

    let content = fs::read_to_string(t.child("src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_ORIGINAL);
    let content = fs::read_to_string(t.child("first/src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_ORIGINAL);
    let content = fs::read_to_string(t.child("second/src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_ORIGINAL);
}
