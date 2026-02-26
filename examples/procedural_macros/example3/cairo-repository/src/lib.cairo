fn main() -> u32 {
    named_wrapper()
}

#[create_wrapper(named_wrapper,16)]
fn fib(mut n: u32) -> u32 {
    let mut a: u32 = 0;
    let mut b: u32 = 1;
    while n != 0 {
        n = n - 1;
        let temp = b;
        b = a + b;
        a = temp;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::main;

    #[test]
    fn it_works() {
        assert(main() == 987, 'invalid value returned!');
    }
}
