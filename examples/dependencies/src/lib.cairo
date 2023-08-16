use alexandria_math::fibonacci::fib;

fn double_fib(a: felt252, b: felt252, n: felt252) -> felt252 {
    2 * fib(a, b, n)
}

#[cfg(test)]
mod tests {
    use super::double_fib;

    #[test]
    #[available_gas(100000)]
    fn it_works() {
        assert(double_fib(0, 1, 16) == 1974, 'it works!');
    }
}
