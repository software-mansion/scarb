#[abi]
trait IBalance {
    fn increase_balance(amount: felt252);
    fn get_balance() -> felt252;
}

#[contract]
mod Balance {
    struct Storage {
        balance: felt252, 
    }

    #[constructor]
    fn constructor(balance_: felt252) {
        balance::write(balance_);
    }

    // Increases the balance by the given amount.
    #[external]
    fn increase_balance(amount: felt252) {
        balance::write(balance::read() + amount);
    }

    // Returns the current balance.
    #[view]
    fn get_balance() -> felt252 {
        balance::read()
    }
}

#[cfg(test)]
mod tests {
    use core::traits::Into;
    use core::result::ResultTrait;
    use starknet::syscalls::deploy_syscall;
    use array::ArrayTrait;
    use traits::TryInto;
    use option::OptionTrait;
    use starknet::class_hash::Felt252TryIntoClassHash;

    use super::{Balance, IBalanceDispatcher, IBalanceDispatcherTrait};

    #[test]
    #[available_gas(30000000)]
    fn test_flow() {
        let mut calldata = ArrayTrait::new();
        calldata.append(100);
        let (contract_address, _) = deploy_syscall(
            Balance::TEST_CLASS_HASH.try_into().unwrap(), 0, calldata.span(), false
        )
            .unwrap();
        let contract = IBalanceDispatcher { contract_address };

        assert(contract.get_balance() == 100, 'contract.get_balance() == 100');
        contract.increase_balance(1);
        assert(contract.get_balance() == 101, 'contract.get_balance() == 101');
    }
}
