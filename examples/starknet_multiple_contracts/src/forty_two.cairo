#[starknet::contract]
mod FortyTwo {
    #[storage]
    struct Storage {}

    #[abi(embed_v0)]
    fn answer(ref self: ContractState) -> felt252 {
        42
    }
}
