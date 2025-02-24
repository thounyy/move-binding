use itertools::Itertools;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use reqwest::header::CONTENT_TYPE;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::str::FromStr;
use sui_types::{Address, ObjectId};
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, ExprArray, LitStr, Path, Token};

const MOVE_STDLIB: &str = "0x0000000000000000000000000000000000000000000000000000000000000001";
const SUI_FRAMEWORK: &str = "0x0000000000000000000000000000000000000000000000000000000000000002";

struct MoveContractArgs {
    sui_env: SuiEnv,
    package_alias: String,
    package_id: String,
    deps: Vec<Path>,
}

impl Parse for MoveContractArgs {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let mut alias = None;
        let mut package = None;
        let mut deps = Vec::new();
        let mut sui_env = SuiEnv::Mainnet;

        while !input.is_empty() {
            let key: Ident = input.parse()?; // Parse the key (e.g., alias, package, deps)
            input.parse::<Token![=]>()?; // Expect '=' token

            if key == "alias" {
                alias = Some(input.parse::<LitStr>()?.value()); // Parse string literal
            } else if key == "package" {
                if let Ok(lit) = input.parse::<LitStr>() {
                    let package_input = lit.value();
                    if package_input.contains("@") || package_input.contains(".sui") {
                        package = resolve_mvr_name(package_input)
                    } else {
                        package = Some(lit.value());
                    }
                }
            } else if key == "deps" {
                let array: ExprArray = input.parse()?; // Parse `[ sui, move_stdlib ]`
                deps = array
                    .elems
                    .iter()
                    .map(|expr| match expr {
                        syn::Expr::Path(path) => path.path.clone(),
                        _ => panic!("Expected an identifier in deps list"),
                    })
                    .collect();
            } else if key == "env" {
                if let Ok(lit) = input.parse::<LitStr>() {
                    sui_env = match lit.value().to_lowercase().as_str() {
                        "mainnet" => SuiEnv::Mainnet,
                        "testnet" => SuiEnv::Testnet,
                        _ => SuiEnv::Custom(lit.value()),
                    };
                }
            } else {
                return Err(syn::Error::new(key.span(), "Unknown key"));
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?; // Consume optional comma
            }
        }

        Ok(MoveContractArgs {
            sui_env,
            package_alias: alias.ok_or_else(|| syn::Error::new(input.span(), "Missing alias"))?,
            package_id: package.ok_or_else(|| syn::Error::new(input.span(), "Missing package"))?,
            deps,
        })
    }
}

#[proc_macro]
pub fn move_contract(input: TokenStream) -> TokenStream {
    let MoveContractArgs {
        sui_env,
        package_alias,
        package_id,
        deps,
    } = parse_macro_input!(input as MoveContractArgs);

    let rpc_url = match sui_env {
        SuiEnv::Mainnet => "https://rpc.mainnet.sui.io:443".to_string(),
        SuiEnv::Testnet => "https://rpc.testnet.sui.io:443".to_string(),
        SuiEnv::Custom(s) => s,
    };

    let client = reqwest::blocking::Client::new();
    let res = client
        .post(rpc_url)
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

    let package_data = res
        .json::<JsonRpcResponse<BTreeMap<String, Value>>>()
        .unwrap()
        .result;

    let module_tokens = package_data.iter().map(|(module_name, module)| {
        let module_ident = Ident::new(module_name, proc_macro2::Span::call_site());
        let structs = module["structs"].as_object().unwrap();
        let module_address = Address::from_str(module["address"].as_str().unwrap()).unwrap();

        let mut struct_tokens = structs
            .iter()
            .map(|(name, move_struct)| {
                let type_parameters = move_struct["typeParameters"].as_array().cloned();
                let (type_parameters, phantoms) =
                    type_parameters.iter().flatten().enumerate().fold(
                        (vec![], vec![]),
                        |(mut type_parameters, mut phantoms), (i, v)| {
                            type_parameters.push(Ident::new(
                                &format!("T{i}"),
                                proc_macro2::Span::call_site(),
                            ));
                            if let Some(true) = v["isPhantom"].as_bool() {
                                let name = Ident::new(
                                    &format!("phantom_data_{i}"),
                                    proc_macro2::Span::call_site(),
                                );
                                let type_: syn::Type =
                                    syn::parse_str(&format!("std::marker::PhantomData<T{i}>"))
                                        .unwrap();
                                phantoms.push(quote! {
                                    #name: #type_,
                                })
                            }
                            (type_parameters, phantoms)
                        },
                    );

                let fields = move_struct["fields"].as_array().unwrap();
                let struct_ident = Ident::new(name, proc_macro2::Span::call_site());
                let field_tokens = fields.iter().map(|field| {
                    let field_ident = Ident::new(
                        &escape_keyword(field["name"].as_str().unwrap().to_string()),
                        proc_macro2::Span::call_site(),
                    );
                    let move_type: MoveType =
                        serde_json::from_value(field["type"].clone()).unwrap();
                    let field_type: syn::Type =
                        syn::parse_str(&move_type.to_rust_type(&package_id, module_name)).unwrap();
                    quote! {
                        pub #field_ident: #field_type,
                    }
                });

                if type_parameters.is_empty() {
                    quote! {
                        #[derive(serde::Deserialize, Debug)]
                        pub struct #struct_ident {
                            #(#field_tokens)*
                        }
                        impl MoveType for #struct_ident {
                            fn type_() -> TypeTag {
                                TypeTag::Struct(Box::new(StructTag {
                                    address: PACKAGE_ID,
                                    module: Identifier::new(MODULE_NAME).unwrap(),
                                    name: Identifier::new(#name).unwrap(),
                                    type_params: vec![],
                                }))
                            }
                        }
                    }
                } else {
                    quote! {
                        #[derive(serde::Deserialize, Debug)]
                        pub struct #struct_ident<#(#type_parameters),*> {
                            #(#field_tokens)*
                            #(#phantoms)*
                        }
                        impl <#(#type_parameters:MoveType),*> MoveType for #struct_ident<#(#type_parameters),*> {
                            fn type_() -> TypeTag {
                                TypeTag::Struct(Box::new(StructTag {
                                    address: PACKAGE_ID,
                                    module: Identifier::new(MODULE_NAME).unwrap(),
                                    name: Identifier::new(#name).unwrap(),
                                    type_params: vec![#(#type_parameters::type_()),*],
                                }))
                            }
                        }
                    }
                }
            })
            .peekable();

        if struct_tokens.peek().is_none() {
            quote! {}
        } else {
            let addr_byte_ident = module_address.as_bytes();
            quote! {
                pub mod #module_ident{
                    use super::*;
                    pub const PACKAGE_ID: Address = Address::new([#(#addr_byte_ident),*]);
                    pub const MODULE_NAME: &str = #module_name;
                    #(#struct_tokens)*
                }
            }
        }
    });

    let package_ident = Ident::new(&package_alias, proc_macro2::Span::call_site());
    let expanded = quote! {
        pub mod #package_ident{
            #(use #deps::*;)*
            use sui_types::{Address, Identifier, StructTag, TypeTag};
            use move_types::MoveType;
            #(#module_tokens)*
        }
    };
    TokenStream::from(expanded)
}

fn resolve_mvr_name(package: String) -> Option<String> {
    let client = reqwest::blocking::Client::new();
    let request = format!(r#"{{packageByName(name:"{package}"){{address}}}}"#);

    let res = client
        .post("https://mvr-rpc.sui-mainnet.mystenlabs.com/graphql")
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "query": request,
            "variables": Value::Null
        }))
        .send()
        .ok()?;
    Some(
        res.json::<Value>().ok()?["data"]["packageByName"]["address"]
            .as_str()?
            .to_string(),
    )
}

enum SuiEnv {
    Mainnet,
    Testnet,
    Custom(String),
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct JsonRpcResponse<T> {
    jsonrpc: String,
    id: u64,
    result: T,
}

fn escape_keyword(mut name: String) -> String {
    match name.as_str() {
        "for" | "ref" => {
            name.push('_');
            name
        }
        _ => name,
    }
}

#[derive(Deserialize, Debug)]
enum MoveType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    Address,
    Signer,
    Struct {
        address: String,
        module: String,
        name: String,
        #[serde(default, alias = "typeArguments")]
        type_arguments: Vec<MoveType>,
    },
    Vector(Box<MoveType>),
    Reference(Box<MoveType>),
    MutableReference(Box<MoveType>),
    TypeParameter(u16),
}

impl MoveType {
    fn to_rust_type(&self, own_package: &str, current_module: &str) -> String {
        match self {
            MoveType::Bool => "bool".to_string(),
            MoveType::U8 => "u8".to_string(),
            MoveType::U16 => "u16".to_string(),
            MoveType::U32 => "u32".to_string(),
            MoveType::U64 => "u64".to_string(),
            MoveType::U128 => "u128".to_string(),
            MoveType::U256 => "u256".to_string(),
            MoveType::Address => "sui_types::Address".to_string(),
            MoveType::Signer => "sui_types::Address".to_string(),
            t @ MoveType::Struct { .. } => t.try_resolve_known_types(own_package, current_module),
            MoveType::Vector(t) => {
                format!("Vec<{}>", t.to_rust_type(own_package, current_module))
            }
            MoveType::Reference(t) => {
                format!("&{}", t.to_rust_type(own_package, current_module))
            }
            MoveType::MutableReference(t) => {
                format!("&mut{}", t.to_rust_type(own_package, current_module))
            }
            MoveType::TypeParameter(index) => format!("T{index}"),
        }
    }

    fn try_resolve_known_types(&self, own_package: &str, current_module: &str) -> String {
        if let MoveType::Struct {
            address,
            module,
            name,
            type_arguments,
        } = self
        {
            // normalise address
            let address = ObjectId::from_str(address).unwrap().to_string();
            let own_package = ObjectId::from_str(own_package).unwrap().to_string();

            match (address.as_str(), module.as_str(), name.as_str()) {
                (MOVE_STDLIB, "type_name", "TypeName") => "String".to_string(),
                (MOVE_STDLIB, "string", "String") => "String".to_string(),
                (MOVE_STDLIB, "ascii", "String") => "String".to_string(),
                (MOVE_STDLIB, "option", "Option") => {
                    format!(
                        "Option<{}>",
                        type_arguments[0].to_rust_type(&own_package, current_module,)
                    )
                }

                (SUI_FRAMEWORK, "object", "UID") => "sui_types::ObjectId".to_string(),
                (SUI_FRAMEWORK, "object", "ID") => "sui_types::ObjectId".to_string(),
                _ => {
                    if type_arguments.is_empty() {
                        format!("{module}::{name}")
                    } else {
                        format!(
                            "{module}::{name}<{}>",
                            type_arguments
                                .iter()
                                .map(|ty| ty.to_rust_type(&own_package, current_module))
                                .join(", ")
                        )
                    }
                }
            }
        } else {
            unreachable!()
        }
    }
}
