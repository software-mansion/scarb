/// Function with a runnable example that fails at compile time.
/// The example calls a function that doesn't exist:
/// ```cairo,runnable
/// undefined();
/// ```
pub fn foo() {
    println!("foo");
}

fn main() {
    println!("hello_world");
}

