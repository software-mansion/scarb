//! Sub-package code (with feature)

/// Function that prints "test" to stdout with endline.
/// Can invoke it like that:
/// ```cairo
///     fn main() {
///         test();
///     }
/// ```
#[cfg(feature: 'test_feature')]
fn test() {
    println!("test");
}

/// Main function that cairo runs as a binary entrypoint.
fn main() {
    println!("hello_world");
}
