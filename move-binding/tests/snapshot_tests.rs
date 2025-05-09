use insta::assert_snapshot;
use move_binding::move_codegen::MoveCodegen;
use move_binding::SuiNetwork;
use syn::parse2;

#[test]
fn test_generate_sui_packages() {
    test_package(SuiNetwork::Mainnet, "0x1", "move_lib");
    test_package(SuiNetwork::Mainnet, "0x2", "sui");
    test_package(SuiNetwork::Mainnet, "0x3", "sui_system");
    test_package(SuiNetwork::Mainnet, "0xb", "bridge");
}

fn test_package(network: SuiNetwork, package: &str, alias: &str) {
    let ts = MoveCodegen::expand(network, package, alias, vec![]).unwrap();
    let file = parse2::<syn::File>(ts.clone()).expect("Failed to parse TokenStream");
    let pretty = prettyplease::unparse(&file);
    assert_snapshot!(package, pretty)
}