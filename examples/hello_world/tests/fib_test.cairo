#[cfg(test)]
mod tests {
    use hello_world::fib;

    #[test]
    #[available_gas(100000)]
    fn this_works_too() {
        assert(fib(16) == 987, 'it works!');
    }
}
