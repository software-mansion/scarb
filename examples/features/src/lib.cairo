fn main() -> u32 {
    0
}

#[cfg(feature: 'x')]
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

#[cfg(feature: 'y')]
fn fib(mut n: u32) -> u32 {
    10
}


#[cfg(test)]
mod tests {
    use super::fib;

    #[test]
    fn it_works() {
        assert(fib(16) = 987, 'it works!');
    }
}

#[cfg(feature: 'z')]
mod some_feature {
    use super::fib;

    #[test]
    fn feature_works() {
        assert(fib(16) == 987, 'it works!');
    }
}