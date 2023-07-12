use indoc::indoc;

pub const BALANCE_CONTRACT: &str = indoc! {r#"
    #[starknet::interface]
    trait IBalance<T> {
        // Returns the current balance.
        fn get(self: @T) -> u128;
        // Increases the balance by the given amount.
        fn increase(ref self: T, a: u128);
    }

    #[starknet::contract]
    mod Balance {
        use traits::Into;

        #[storage]
        struct Storage {
            value: u128,
        }

        #[constructor]
        fn constructor(ref self: ContractState, value_: u128) {
            self.value.write(value_);
        }

        #[external(v0)]
        impl Balance of super::IBalance<ContractState> {
            fn get(self: @ContractState) -> u128 {
                self.value.read()
            }
            fn increase(ref self: ContractState, a: u128)  {
                self.value.write( self.value.read() + a );
            }
        }
    }
"#};

pub const FORTY_TWO_CONTRACT: &str = indoc! {r#"
    #[starknet::interface]
    trait IFortyTwo<T> {
        fn answer(ref self: T) -> felt252;
    }
    #[starknet::contract]
    mod FortyTwo {
        #[storage]
        struct Storage {}
        #[external(v0)]
        fn answer(ref self: ContractState) -> felt252 { 42 }
        impl FortyTwo of super::IFortyTwo<ContractState> {
            fn answer(ref self: ContractState) -> felt252 { 42 }
        }
    }
"#};

pub const HELLO_CONTRACT: &str = indoc! {r#"
    #[starknet::interface]
    trait IHelloContract<T> {
        fn answer(ref self: T) -> felt252;
    }
    #[starknet::contract]
    mod HelloContract {
        #[storage]
        struct Storage {}
        #[external(v0)]
        impl HelloContract of super::IHelloContract<ContractState> {
            fn answer(ref self: ContractState) -> felt252 { 'hello' }
        }
    }
"#};
