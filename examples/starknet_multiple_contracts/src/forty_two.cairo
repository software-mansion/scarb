#[starknet::interface]
pub trait IFortyTwo<TContractState> {
    fn answer(self: @TContractState) -> felt252;
}

#[starknet::contract]
mod FortyTwo {
    #[storage]
    struct Storage {}

    #[abi(embed_v0)]
    impl FortyTwo of super::IFortyTwo<ContractState> {
        fn answer(self: @ContractState) -> felt252 {
            42
        }
    }
}
