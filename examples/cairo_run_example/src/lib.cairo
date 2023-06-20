fn main() -> u128 {
    fib(1_u128, 1_u128, 2_u128)
}

fn fib(a: u128, b: u128, n: u128) -> u128 {
    if n == 0 {
        a
    } else {
        fib(b, a + b, n - 1_u128)
    }
}
