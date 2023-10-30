fn add(a: felt252, b: felt252) -> felt252 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::add;

    #[test]
    fn it_works() {
        assert(add(2, 3) == 5, 'it works!');
    }
}
