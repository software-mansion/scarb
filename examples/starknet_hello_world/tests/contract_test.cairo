#[cfg(test)]
mod tests {
    use array::ArrayTrait;
    use core::result::ResultTrait;
    use core::traits::Into;
    use option::OptionTrait;
    use starknet::syscalls::deploy_syscall;
    use traits::TryInto;

    use test::test_utils::assert_eq;

    use starknet_hello_world::{Balance, IBalance, IBalanceDispatcher, IBalanceDispatcherTrait};

    #[test]
    fn test_flow() {
        let calldata = array![100];
        let (address0, _) = deploy_syscall(
            Balance::TEST_CLASS_HASH.try_into().unwrap(), 0, calldata.span(), false
        )
            .unwrap();
        let mut contract0 = IBalanceDispatcher { contract_address: address0 };

        let calldata = array![200];
        let (address1, _) = deploy_syscall(
            Balance::TEST_CLASS_HASH.try_into().unwrap(), 0, calldata.span(), false
        )
            .unwrap();
        let mut contract1 = IBalanceDispatcher { contract_address: address1 };

        assert_eq(@contract0.get(), @100, 'contract0.get() == 100');
        assert_eq(@contract1.get(), @200, 'contract1.get() == 200');
        @contract1.increase(200);
        assert_eq(@contract0.get(), @100, 'contract0.get() == 100');
        assert_eq(@contract1.get(), @400, 'contract1.get() == 400');
    }
}
