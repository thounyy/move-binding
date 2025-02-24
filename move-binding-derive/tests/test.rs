use std::str::FromStr;
use sui_client::Client;
use sui_sdk_types::{Address, ObjectData};

use crate::bridge::bridge::BridgeInner;
use crate::sui::coin::Coin;
use crate::sui::dynamic_field::Field;
use crate::sui::sui::SUI;
use crate::sui::vec_map::VecMap;
use move_binding_derive::move_contract;
use move_types::MoveType;

move_contract! {alias = "sui", package = "0x2"}
move_contract! {alias = "bridge", package = "0xb", deps = [crate::sui]}

move_contract! {alias = "mvr_metadata", package = "@mvr/metadata", deps = [crate::sui]}
move_contract! {alias = "suins", package = "0xd22b24490e0bae52676651b4f56660a5ff8022a2576e0089f79b3c88d44e08f0", deps = [crate::sui]}
move_contract! {alias = "mvr_core", package = "@mvr/core", deps = [crate::sui, crate::suins]}

move_contract! {alias = "mvr_metadata_testnet", package = "@mvr/metadata", network = "testnet", deps = [crate::sui]}

#[tokio::test]
pub async fn test() {
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
    }

    println!("{}", Coin::<SUI>::type_());
    println!("{}", VecMap::<u64, SUI>::type_());
}
