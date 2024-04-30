use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};

#[test]
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

    let expected_items = [
        "main",
        "FOO",
        "fib",
        "Pair",
        "Color",
        "Shape",
        "Circle",
        "CircleShape",
        "tests",
        "fib_function",
        "it_works",
    ];
    for item in expected_items.iter() {
        assert!(stdout.contains(item));
    }
}
