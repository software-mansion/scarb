/// Example that fails to compile.
/// ```cairo,runnable,compile_fail
/// is_odd(true);
/// ```
/// Example that unexpectedly compiles.
/// ```cairo,runnable,compile_fail
/// let result = is_odd(3);
/// println!("{}", result);
/// ```
pub fn is_odd(n: i32) -> bool {
    n % 2 != 0
}

fn main() {
    println!("hello_world");
}

