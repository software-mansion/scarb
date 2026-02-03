/// Function with a runnable example that fails at runtime.
/// The example panics:
/// ```cairo,runnable
/// foo();
/// ```
pub fn foo() {
    panic!("Runtime error occurred");
}

fn main() {
    println!("hello_world");
}

