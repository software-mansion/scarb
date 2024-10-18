use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};

#[test]
fn test_diagnostics_success() {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
      fn main() {
        println!("Hello world!");
      }
    "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("doc")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn test_diagnostics_with_error_code() {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
          fn main() {
            println!("Hello world!");
            wrong code
          }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("doc")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn test_diagnostics_allowed_warnings() {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
          fn main() {
            println!("Hello world!");
            let a = 5;
          }
        "#})
        .manifest_extra(indoc! {r#"
        [cairo]
        allow-warnings = true
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("doc")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn test_diagnostics_not_allowed_warnings() {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
          fn main() {
            println!("Hello world!");
            let a = 5;
          }
        "#})
        .manifest_extra(indoc! {r#"
        [cairo]
        allow-warnings = false
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("doc")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn test_diagnostics_error() {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
            #[starknet::contract]
            pub(crate) mod DualCaseERC20Mock {
                use starknet::ContractAddress;

                component!(path: ERC20Component, storage: erc20, event: ERC20Event);

                #[storage]
                pub struct Storage {
                    #[substorage(v0)]
                    pub erc20: ERC20Component::Storage
                }

                #[event]
                enum Event {
                    #[flat]
                    ERC20Event: ERC20Component::Event
                }
            }
          }
        "#})
        .build(&t);

    let snapbox = Scarb::quick_snapbox()
        .arg("doc")
        .current_dir(&t)
        .assert()
        .failure();

    #[cfg(windows)]
      snapbox.stdout_matches(indoc! {r#"
    error: Skipped tokens. Expected: Const/Enum/ExternFunction/ExternType/Function/Impl/InlineMacro/Module/Struct/Trait/TypeAlias/Use or an attribute.
     --> [..]
    }
    ^

    error: Identifier not found.
     --> [..]
              ERC20Event: ERC20Component::Event
                          ^************^

    error: Identifier not found.
     --> [..]
              pub erc20: ERC20Component::Storage
                         ^************^

    error: Type annotations needed. Failed to infer ?0.
     --> [..]
          #[storage]
          ^********^

    error: Invalid drop trait implementation, Trait `core::traits::Drop::<<missing>>` has multiple implementations, in: `hello_world::DualCaseERC20Mock::ContractStateDrop`, `hello_world::DualCaseERC20Mock::StorageStorageBaseDrop`
     --> [..]
          #[storage]
          ^********^

    error: Trait has no implementation in context: core::starknet::event::Event::<hello_world::DualCaseERC20Mock::Event>.
     --> [..]
      #[starknet::contract]
      ^*******************^

    error: Identifier not found.
     --> [..]
          component!(path: ERC20Component, storage: erc20, event: ERC20Event);
                           ^************^

    error: Invalid drop trait implementation, Candidate impl core::starknet::storage::storage_base::FlattenedStorageDrop::<?0> has an unused generic parameter.
     --> [..]
          #[storage]
          ^********^

    error: Invalid copy trait implementation, Candidate impl core::starknet::storage::storage_base::FlattenedStorageCopy::<?0> has an unused generic parameter.
     --> [..]
          #[storage]
          ^********^

    error: Compilation failed.
    error: process did not exit successfully: exit code: 1
    "#});

    #[cfg(not(windows))]
    snapbox.stdout_matches(indoc! {r#"
    error: Skipped tokens. Expected: Const/Enum/ExternFunction/ExternType/Function/Impl/InlineMacro/Module/Struct/Trait/TypeAlias/Use or an attribute.
     --> [..]
    }
    ^

    error: Identifier not found.
     --> [..]
              ERC20Event: ERC20Component::Event
                          ^************^

    error: Identifier not found.
     --> [..]
              pub erc20: ERC20Component::Storage
                         ^************^

    error: Type annotations needed. Failed to infer ?0.
     --> [..]
          #[storage]
          ^********^

    error: Invalid drop trait implementation, Trait `core::traits::Drop::<<missing>>` has multiple implementations, in: `hello_world::DualCaseERC20Mock::ContractStateDrop`, `hello_world::DualCaseERC20Mock::StorageStorageBaseDrop`
     --> [..]
          #[storage]
          ^********^

    error: Trait has no implementation in context: core::starknet::event::Event::<hello_world::DualCaseERC20Mock::Event>.
     --> [..]
      #[starknet::contract]
      ^*******************^

    error: Identifier not found.
     --> [..]
          component!(path: ERC20Component, storage: erc20, event: ERC20Event);
                           ^************^

    error: Invalid drop trait implementation, Candidate impl core::starknet::storage::storage_base::FlattenedStorageDrop::<?0> has an unused generic parameter.
     --> [..]
          #[storage]
          ^********^

    error: Invalid copy trait implementation, Candidate impl core::starknet::storage::storage_base::FlattenedStorageCopy::<?0> has an unused generic parameter.
     --> [..]
          #[storage]
          ^********^

    error: Compilation failed.
  "#});
}
