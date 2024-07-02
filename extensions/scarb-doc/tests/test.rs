use assert_fs::{fixture::PathChild, TempDir};
use indoc::indoc;
use scarb_test_support::{
    command::Scarb, project_builder::ProjectBuilder, workspace_builder::WorkspaceBuilder,
};

#[test]
#[ignore = "TODO(piotmag769): fix"]
fn test_main() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
        //! Fibonacci sequence calculator

        /// Main function that calculates the 16th Fibonacci number
        fn main() -> u32 {
            fib(16)
        }

        /// FOO constant with value 42
        const FOO: u32 = 42;

        /// Calculate the nth Fibonacci number
        ///
        /// # Arguments
        /// * `n` - The index of the Fibonacci number to calculate
        /// 
        fn fib(mut n: u32) -> u32 {
            let mut a: u32 = 0;
            let mut b: u32 = 1;
            while n != 0 {
                n = n - 1;
                let temp = b;
                b = a + b;
                a = temp;
            };
            a
        }

        /// Pair type alias for a tuple of two u32 values
        type Pair = (u32, u32);

        /// Color enum with Red, Green, and Blue variants
        enum Color {
            /// Red color
            Red: (),
            /// Green color
            Green: (),
            /// Blue color
            Blue: (),
        }

        /// Shape trait for objects that have an area
        trait Shape<T> {
            /// Calculate the area of the shape
            fn area(self: T) -> u32;
        }

        /// Circle struct with radius field
        #[derive(Drop, Serde, PartialEq)]
        struct Circle {
            /// Radius of the circle
            radius: u32,
        }

        /// Implementation of the Shape trait for Circle
        impl CircleShape of Shape<Circle> {
            fn area(self: Circle) -> u32 {
                3 * self.radius * self.radius
            }
        }

        /// Tests module
        mod tests {
            /// Imported fib function from the parent module
            use super::fib as fib_function;

            #[test]
            fn it_works() {
                assert(fib_function(16) == 987, 'it works!');
            }
        }
        "#})
        .build(&t);

    let output = Scarb::quick_snapbox()
        .arg("doc")
        .current_dir(&t)
        .assert()
        .success();
    let stdout = std::str::from_utf8(&output.get_output().stdout).unwrap();

    assert_eq!(
        stdout,
        indoc! {r#"
        Module: hello_world
        Submodules      : ["hello_world::tests"]
        Constants       : ["FOO"]
        Uses            : []
        Free Functions  : ["main", "fib"]
        Structs         : ["Circle"]
        Enums           : ["Color"]
        Type Aliases    : ["Pair"]
        Impl Aliases    : []
        Traits          : ["Shape"]
        Impls           : ["CircleShape", "CircleDrop", "CircleSerde", "CirclePartialEq"]
        Extern Types    : []
        Extern Functions: []
        
        Module: hello_world::tests
        Submodules      : []
        Constants       : []
        Uses            : ["fib_function"]
        Free Functions  : ["it_works"]
        Structs         : []
        Enums           : []
        Type Aliases    : []
        Impl Aliases    : []
        Traits          : []
        Impls           : []
        Extern Types    : []
        Extern Functions: []
        "#}
    )
}

#[test]
#[ignore = "TODO(piotmag769): fix"]
fn test_workspace() {
    let t = TempDir::new().unwrap();
    let hello = t.child("hello_world");
    let goodbye = t.child("goodbye_world");

    ProjectBuilder::start()
        .name("hello_world")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
        /// Hello world
        fn hello_world() -> u32 {
            1
        }
        "#})
        .build(&hello);

    ProjectBuilder::start()
        .name("goodbye_world")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
        /// Goodbye world
        fn goodbye_world() -> u32 {
            0
        }
        "#})
        .build(&goodbye);

    WorkspaceBuilder::start()
        .add_member("hello_world")
        .add_member("goodbye_world")
        .build(&t);

    let output = Scarb::quick_snapbox()
        .arg("doc")
        .arg("-p")
        .arg("hello_world")
        .current_dir(&t)
        .assert()
        .success();
    let stdout = std::str::from_utf8(&output.get_output().stdout).unwrap();
    assert!(stdout.contains("Module: hello_world"));
    assert!(!stdout.contains("Module: goodbye_world"));
}
