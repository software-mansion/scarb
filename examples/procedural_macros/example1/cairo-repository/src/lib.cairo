fn main() -> u32 {
    fib!(16)
}

#[cfg(test)]
mod tests {
    use super::main;

    #[test]
    fn it_works() {
        assert(main() == 987, 'invalid value returned!');
    }
}
