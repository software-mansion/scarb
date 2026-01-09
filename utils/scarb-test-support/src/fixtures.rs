use crate::project_builder::ProjectBuilder;
use assert_fs::TempDir;
use indoc::indoc;

pub fn executable_project_builder() -> ProjectBuilder {
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]

            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
                42
            }
        "#})
}

pub fn build_executable_project() -> TempDir {
    let t = TempDir::new().unwrap();
    executable_project_builder().build(&t);
    t
}
