/// Private, parallel declaration of `starknet::testing::cheatcode`.
///
/// We roll out our own so that oracles are not dependent on the `starknet` package.
pub extern fn cheatcode<const selector: felt252>(
    input: Span<felt252>,
) -> Span<felt252> implicits() nopanic;
