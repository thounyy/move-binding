use crate::bridge::bridge::BridgeInner;
use crate::sui::coin::Coin;
use crate::sui::dynamic_field::Field;
use crate::sui::sui::SUI;
use crate::sui::vec_map::VecMap;
use crate::test_pkg::test_mod::Test;
use move_binding_derive::move_contract;
use move_types::{Key, MoveType};
use std::str::FromStr;
use sui_client::Client;
use sui_sdk_types::{Address, ObjectData, ObjectId};
use sui_transaction_builder::unresolved::Input;
use sui_transaction_builder::{Serialized, TransactionBuilder};

move_contract! {alias = "sui", package = "0x2"}
move_contract! {alias = "sui_system", package = "0x3", deps = [crate::sui]}
move_contract! {alias = "bridge", package = "0xb", deps = [crate::sui, crate::sui_system]}

move_contract! {alias = "mvr_metadata", package = "@mvr/metadata", deps = [crate::sui]}
move_contract! {alias = "suins", package = "0xd22b24490e0bae52676651b4f56660a5ff8022a2576e0089f79b3c88d44e08f0", deps = [crate::sui]}
move_contract! {alias = "mvr_core", package = "@mvr/core", deps = [crate::sui, crate::suins, crate::mvr_metadata]}

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
        println!("{:?}", bridge.id());
    }
    let owner = Address::from_str("0xc2c4885770f4cd16f59ade61d3b5f6b8201e850d1fb2075ff972085f1af2f4f7").unwrap();
    let gas = ObjectId::from_str("0x00b72cdc6688ec1891c4a118aa3a5aba43ea101616395a474ef123d4e451ab7c").unwrap();
    let gas = client.object(gas.into(), None).await.unwrap().unwrap();

    let mut builder = TransactionBuilder::new();
    let owner_input = builder.input(Serialized(&owner));

    let new_bag = sui::bag::new(&mut builder);
    sui::transfer::public_transfer(&mut builder, new_bag, owner_input.into());

    builder.set_sender(owner);
    builder.add_gas_objects(vec![Input::owned(gas.object_id(), gas.version(), gas.digest())]);
    builder.set_gas_budget(10000000);
    builder.set_gas_price(1000);
    let tx = builder.finish().unwrap();

    let result = client.dry_run_tx(&tx, Some(false)).await.unwrap();

    println!("{:?}", result);


    println!("{}", Coin::<SUI>::type_());
    println!("{}", VecMap::<u64, SUI>::type_());

    println!("{}", Test::<SUI>::type_())
}

pub mod test_pkg {
    pub mod test_mod {
        use move_binding_derive::MoveStruct;
        use move_types::functions::{Arg, Ref};
        use move_types::Address;
        use move_types::MoveType;
        use std::marker::PhantomData;
        use std::str::FromStr;
        use sui_sdk_types::Identifier;
        use sui_transaction_builder::{Function, TransactionBuilder};

        pub const PACKAGE_ID: Address = Address::new([0; 32]);
        pub const MODULE_NAME: &str = "test";

        #[derive(MoveStruct)]
        pub struct Test<T0> {
            phantom_data: PhantomData<T0>,
        }

        pub trait MoveModule {
            fn move_fun1<T0: MoveType, T1: MoveType>(&mut self, input: Ref<T0>) -> Arg<u64>;
            fn move_fun2(&mut self) -> String;
        }
        impl MoveModule for TransactionBuilder {
            fn move_fun1<T0: MoveType, T1: MoveType>(&mut self, input: Ref<T0>) -> Arg<u64> {
                self.move_call(
                    Function::new(
                        PACKAGE_ID,
                        Identifier::from_str(MODULE_NAME).unwrap(),
                        Identifier::from_str("").unwrap(),
                        vec![T0::type_(), T1::type_()],
                    ),
                    vec![input.into()],
                )
                    .into()
            }
            fn move_fun2(&mut self) -> String {
                todo!()
            }
        }
    }
}
