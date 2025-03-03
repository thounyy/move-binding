use crate::{MoveType, SuiNetwork};
use reqwest::header::CONTENT_TYPE;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use sui_sdk_types::Address;

pub trait ModuleProvider {
    fn get_modules(&self, package_id: Address) -> HashMap<String, MoveModule>;
}

pub struct RPCModuleProvider {
    network: SuiNetwork,
}

impl RPCModuleProvider {
    pub fn new(network: SuiNetwork) -> Self {
        Self { network }
    }
}

impl ModuleProvider for RPCModuleProvider {
    fn get_modules(&self, package_id: Address) -> HashMap<String, MoveModule> {
        let client = reqwest::blocking::Client::new();
        let res = client
            .post(self.network.rpc())
            .header(CONTENT_TYPE, "application/json")
            .body(format!(
                r#"
                {{
                  "jsonrpc": "2.0",
                  "id": 1,
                  "method": "sui_getNormalizedMoveModulesByPackage",
                  "params": [
                    "{package_id}"
                  ]
                }}
                "#
            ))
            .send()
            .unwrap();

        let value = res.json::<Value>().unwrap();

        serde_json::from_value(value["result"].clone()).unwrap()
    }
}

#[derive(Deserialize)]
pub struct MoveModule {
    pub structs: HashMap<String, MoveStruct>,
    #[serde(alias = "exposedFunctions")]
    pub exposed_functions: HashMap<String, MoveFunction>,
    pub address: Address,
}

#[derive(Deserialize, Debug)]
pub struct MoveStruct {
    pub fields: Vec<MoveStructField>,
    #[serde(alias = "typeParameters")]
    pub type_parameters: Vec<MoveStructTypeParameter>,
    pub abilities: MoveAbilities,
}

#[derive(Deserialize, PartialEq, Debug)]
pub enum MoveAbility {
    Key,
    Copy,
    Store,
    Drop,
}

#[derive(Deserialize, PartialEq, Debug)]
pub struct MoveAbilities {
    pub abilities: Vec<MoveAbility>,
}

impl MoveAbilities {
    pub fn has_key(&self) -> bool {
        self.abilities.contains(&MoveAbility::Key)
    }
}

#[derive(Deserialize, Debug)]
pub struct MoveStructField {
    pub name: String,
    #[serde(alias = "type")]
    pub type_: MoveType,
}

#[derive(Deserialize, Debug)]
pub struct MoveFunction {
    pub parameters: Vec<MoveType>,
    #[serde(alias = "typeParameters")]
    pub type_parameters: Vec<MoveAbilities>,
    #[serde(alias = "return")]
    pub return_: Vec<MoveType>,
}

#[derive(Deserialize, Debug)]
pub struct MoveStructTypeParameter {
    #[serde(default, alias = "isPhantom")]
    pub is_phantom: bool,
    //pub constraints: MoveAbilities,
}
