use crate::bridge::bridge::BridgeInner;
use crate::sui::dynamic_field::Field;
use move_binding_derive::move_contract;
use move_types::Key;
use std::str::FromStr;
use sui_client::Client;
use sui_sdk_types::{Address, ObjectData, ObjectId};
use sui_transaction_builder::unresolved::Input;
use sui_transaction_builder::TransactionBuilder;

move_contract! {alias = "move_lib", package = "0x1"}
move_contract! {alias = "sui", package = "0x2"}
move_contract! {alias = "sui_system", package = "0x3", deps = [crate::sui]}
move_contract! {alias = "bridge", package = "0xb", deps = [crate::sui, crate::sui_system]}

move_contract! {alias = "mvr_metadata", package = "@mvr/metadata", deps = [crate::sui]}
move_contract! {alias = "suins", package = "0xd22b24490e0bae52676651b4f56660a5ff8022a2576e0089f79b3c88d44e08f0", deps = [crate::sui]}
move_contract! {alias = "mvr_core", package = "@mvr/core", deps = [crate::sui, crate::suins, crate::mvr_metadata]}

move_contract! {alias = "mvr_metadata_testnet", package = "@mvr/metadata", network = "testnet", deps = [crate::sui]}

#[tokio::test]
pub async fn test_deserialize_object() {
    let client = Client::new("https://sui-mainnet.mystenlabs.com/graphql").unwrap();
    let bridge_obj = client
        .object(
            Address::from_str(
                "0x00ba8458097a879607d609817a05599dc3e9e73ce942f97d4f1262605a8bf0fc".into(),
            )
            .unwrap(),
            None,
        )
        .await
        .unwrap()
        .unwrap();

    if let ObjectData::Struct(o) = bridge_obj.data() {
        let bridge: Field<u64, BridgeInner> = bcs::from_bytes(o.contents()).unwrap();
        println!("{:?}", bridge);
        println!("{:?}", bridge.id());
    }
}

#[tokio::test]
pub async fn test_function_call() {
    let client = Client::new("https://sui-mainnet.mystenlabs.com/graphql").unwrap();

    let owner = Address::from_str("0x2").unwrap();
    let gas =
        ObjectId::from_str("0x726b714a3c4c681d8a9b1ff1833ad368585579a273362e1cbd738c0c8f70dabd")
            .unwrap();
    let gas = client.object(gas.into(), None).await.unwrap().unwrap();

    let mut builder = TransactionBuilder::new();
    builder.set_sender(owner);
    builder.add_gas_objects(vec![Input::owned(
        gas.object_id(),
        gas.version(),
        gas.digest(),
    )]);
    builder.set_gas_budget(10000000);
    builder.set_gas_price(1000);

    let mut new_bag = sui::bag::new(&mut builder);
    sui::bag::add(
        &mut builder,
        new_bag.borrow_mut(),
        "Test".into(),
        "Test_value".into(),
    );
    sui::bag::add(
        &mut builder,
        new_bag.borrow_mut(),
        "Test2".into(),
        "Test_value2".into(),
    );
    sui::transfer::public_transfer(&mut builder, new_bag, owner.into());

    let tx = builder.finish().unwrap();
    let result = client.dry_run_tx(&tx, None).await.unwrap();

    println!("{:?}", result);
}
