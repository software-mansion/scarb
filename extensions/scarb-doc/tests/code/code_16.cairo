/// Example that panics.
/// ```cairo,runnable,should_panic
/// assert(is_odd(2), '2 is not odd');
/// ```
/// Example that unexpectedly does not panic.
/// ```cairo,runnable,should_panic
/// let result = is_odd(3);
/// assert(result, '3 is odd');
/// ```
pub fn is_odd(n: i32) -> bool {
    n % 2 != 0
}

fn main() {
    println!("hello_world");
}

