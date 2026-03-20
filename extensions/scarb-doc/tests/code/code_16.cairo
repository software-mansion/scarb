/// A simple storage contract with a runnable example.
/// ```cairo,runnable
/// let balance = 100_u128;
/// assert!(balance > 0);
/// ```
#[starknet::interface]
pub trait IBalance<T> {
    fn get(self: @T) -> u128;
    fn increase(ref self: T, a: u128);
}

#[starknet::contract]
pub mod Balance {
    #[storage]
    struct Storage {
        value: u128,
    }

    #[constructor]
    fn constructor(ref self: ContractState, value_: u128) {
        self.value.write(value_);
    }

    #[abi(embed_v0)]
    impl Balance of super::IBalance<ContractState> {
        fn get(self: @ContractState) -> u128 {
            self.value.read()
        }
        fn increase(ref self: ContractState, a: u128) {
            self.value.write(self.value.read() + a);
        }
    }
}
