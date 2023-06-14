#[starknet::contract]
mod FortyTwo {
    #[storage]
    struct Storage {}

    #[external(v0)]
    fn answer(ref self: ContractState) -> felt252 {
        42
    }
}
