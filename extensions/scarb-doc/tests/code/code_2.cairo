//! Sub-package code (without feature)

/// Function that prints "test" to stdout with endline.
/// Can invoke it like that:
/// ```cairo
///     fn main() {
///         test();
///     }
/// ```
fn test() {
    //! Don't forget this function prints to stdout.
    println!("test");
}

/// Main function that cairo runs as a binary entrypoint.
fn main() {
    //! Entry point of binary.
    println!("hello_world");
}
