use assert_fs::TempDir;
use assert_fs::fixture::{FileWriteStr, PathChild};
use indoc::{formatdoc, indoc};
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;
use test_case::test_case;

#[test_case(
    "standalone",
    "1129815197211541481934112806673325772687763881719835256646064516195041515616"
)]
#[test_case(
    "bootloader",
    "1\n3\n-1272769738511508193392446484420387256363817420616635924344207712676117344847\n1129815197211541481934112806673325772687763881719835256646064516195041515616"
)]
fn can_take_big_number_as_arg(target: &str, output: &str) {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [executable]
            
            [cairo]
            enable-gas = false
        "#})
        .dep_cairo_execute()
        .lib_cairo(indoc! {r#"
        #[executable]
        fn main(a: felt252, b: felt252) -> felt252 {
            b
        }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("execute")
        .arg("--print-program-output")
        .arg(format!("--target={target}"))
        .arg("--arguments")
        .arg(r#"1,1129815197211541481934112806673325772687763881719835256646064516195041515616"#)
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(formatdoc! {r#"
            [..]Compiling hello v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing hello
            Program output:
            {output}
            Saving output to: target/execute/hello/execution1
        "#});
}

#[test_case(
    "standalone",
    "1129815197211541481934112806673325772687763881719835256646064516195041515616"
)]
#[test_case(
    "bootloader",
    "1\n3\n-1272769738511508193392446484420387256363817420616635924344207712676117344847\n1129815197211541481934112806673325772687763881719835256646064516195041515616"
)]
fn can_read_arguments_from_file(target: &str, output: &str) {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [executable]

            [cairo]
            enable-gas = false
        "#})
        .dep_cairo_execute()
        .lib_cairo(indoc! {r#"
        #[executable]
        fn main(a: felt252, b: felt252) -> felt252 {
            b
        }
        "#})
        .build(&t);

    t.child("args.txt")
        .write_str(r#"["0x1","0x27F73E6C94FA8249EC9F2F4EEC607ACC97FA632C9E8FB6C49437E62390D9860"]"#)
        .unwrap();

    Scarb::quick_command()
        .arg("execute")
        .arg("--print-program-output")
        .arg(format!("--target={target}"))
        .args(["--arguments-file", "args.txt"])
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(formatdoc! {r#"
            [..]Compiling hello v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing hello
            Program output:
            {output}
            Saving output to: target/execute/hello/execution1
        "#});
}
