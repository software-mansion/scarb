// HACK: We need integration tests, for cargo test to generate the binary "scarb-test-support",
// necessary for cargo_bin("scarb-test-support") to work correctly
// (used in ctrl_c_kills_everyone test in scarb/tests/subcommand.rs).

#[test]
fn binary_dependencies_hack() {}
