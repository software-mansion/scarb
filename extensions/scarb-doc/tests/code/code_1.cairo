//! Fibonacci sequence calculator

/// Main function that calculates the 16th Fibonacci number
fn main() -> u32 {
    fib(16)
}

/// use into_trait
use core::traits::Into as into_trait;
use core::traits::TryInto;

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
    /// Constant for the shape type
    const SHAPE_CONST: felt252;

    /// Type alias for a pair of shapes
    type ShapePair;

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
    /// Type alias for a pair of circles
    type ShapePair = (Circle, Circle);

    /// Shape constant
    const SHAPE_CONST: felt252 = 'xyz';

    /// Implementation of the area method for Circle
    fn area(self: Circle) -> u32 {
        3 * self.radius * self.radius
    }
}

/// Tests module
mod tests {
    /// Imported fib function from the parent module
    use super::fib as fib_function;

    /// Really
    /// works.
    fn it_works() {
        assert(fib_function(16) == 987, 'it works!');
    }
}
