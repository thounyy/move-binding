mod package_provider;
use crate::package_provider::{ModuleProvider, MoveFunction, MoveStruct, RPCModuleProvider};
use itertools::Itertools;
use move_types::MOVE_STDLIB;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use reqwest::header::CONTENT_TYPE;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::str::FromStr;
use sui_sdk_types::{Address, Identifier};
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, DeriveInput, ExprArray, GenericParam, Generics, LitStr, Path, Token};

#[proc_macro_derive(Key)]
pub fn key_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;

    let types = extract_type_ident(&ast.generics);

    let gen = if types.is_empty() {
        quote! {
            impl move_types::Key for #name {
                fn id(&self) -> &move_types::ObjectId {
                    &self.id
                }
            }
        }
    } else {
        quote! {
            impl <#(#types:move_types::MoveType),*> move_types::Key for #name<#(#types),*> {
                fn id(&self) -> &move_types::ObjectId {
                    &self.id
                }
            }
        }
    };
    gen.into()
}

fn extract_type_ident(generics: &Generics) -> Vec<Ident> {
    generics
        .params
        .iter()
        .flat_map(|p| {
            if let GenericParam::Type(t) = p {
                Some(t.ident.clone())
            } else {
                None
            }
        })
        .collect()
}

#[proc_macro_derive(MoveStruct)]
pub fn move_struct_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;

    let types = extract_type_ident(&ast.generics);
    let name_str = name.to_string();

    let gen = if types.is_empty() {
        quote! {
            impl move_types::MoveStruct for #name {
                fn struct_type() -> move_types::StructTag {
                    move_types::StructTag {
                        address: PACKAGE_ID,
                        module: move_types::Identifier::from_str(MODULE_NAME).unwrap(),
                        name: move_types::Identifier::from_str(#name_str).unwrap(),
                        type_params: vec![],
                    }
                }
            }
        }
    } else {
        quote! {
            impl <#(#types:move_types::MoveType), *> move_types::MoveStruct for #name<#(#types),*> {
                fn struct_type() -> move_types::StructTag {
                    move_types::StructTag {
                        address: PACKAGE_ID,
                        module: move_types::Identifier::from_str(MODULE_NAME).unwrap(),
                        name: move_types::Identifier::from_str(#name_str).unwrap(),
                        type_params: vec![#(#types::type_()),*],
                    }
                }
            }
        }
    };
    gen.into()
}

struct MoveContractArgs {
    network: SuiNetwork,
    package_alias: String,
    package: String,
    deps: Vec<Path>,
}

impl Parse for MoveContractArgs {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let mut alias = None;
        let mut package = None;
        let mut deps = Vec::new();
        let mut network = SuiNetwork::Mainnet;

        while !input.is_empty() {
            let key: Ident = input.parse()?; // Parse the key (e.g., alias, package, deps)
            input.parse::<Token![=]>()?; // Expect '=' token

            if key == "alias" {
                alias = Some(input.parse::<LitStr>()?.value()); // Parse string literal
            } else if key == "package" {
                package = Some(input.parse::<LitStr>()?.value()); // Parse string literal
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
            } else if key == "network" {
                if let Ok(lit) = input.parse::<LitStr>() {
                    network = match lit.value().to_lowercase().as_str() {
                        "mainnet" => SuiNetwork::Mainnet,
                        "testnet" => SuiNetwork::Testnet,
                        _ => {
                            return Err(syn::Error::new(
                                key.span(),
                                "Unknown network, only ['mainnet', 'testnet'] are supported.",
                            ))
                        }
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
            network,
            package_alias: alias.ok_or_else(|| syn::Error::new(input.span(), "Missing alias"))?,
            package: package.ok_or_else(|| syn::Error::new(input.span(), "Missing package"))?,
            deps,
        })
    }
}

#[proc_macro]
pub fn move_contract(input: TokenStream) -> TokenStream {
    let MoveContractArgs {
        network,
        package_alias,
        package,
        deps,
    } = parse_macro_input!(input as MoveContractArgs);

    let package_id = if package.contains("@") || package.contains(".sui") {
        resolve_mvr_name(package, &network.gql()).expect("Cannot resolve mvr name")
    } else {
        Address::from_str(&package).expect("Error parsing package id.")
    };

    let module_provider = RPCModuleProvider::new(network);
    let modules = module_provider.get_modules(package_id);

    let module_tokens = modules.iter().map(|(module_name, module)| {
        let module_ident = Ident::new(module_name, proc_macro2::Span::call_site());
        let mut struct_fun_tokens = create_structs(&module.structs);

        if !module.exposed_functions.is_empty() {
            let fun_impl = create_funs(&module.exposed_functions);
            struct_fun_tokens.extend(fun_impl);
        }

        if struct_fun_tokens.is_empty() {
            quote! {}
        } else {
            let addr_byte_ident = module.address.as_bytes();
            quote! {
                pub mod #module_ident{
                    use super::*;
                    pub const PACKAGE_ID: Address = Address::new([#(#addr_byte_ident),*]);
                    pub const MODULE_NAME: &str = #module_name;
                    #(#struct_fun_tokens)*
                }
            }
        }
    });

    let package_ident = Ident::new(&package_alias, proc_macro2::Span::call_site());
    let expanded = quote! {
        pub mod #package_ident{
            #(use #deps::*;)*
            use std::str::FromStr;
            use move_binding_derive::{Key, MoveStruct};
            use move_types::{MoveType, Address, Identifier, TypeTag, StructTag};
            use move_types::functions::{Arg, Ref, MutRef};
            #(#module_tokens)*
        }
    };
    expanded.into()
}

fn resolve_mvr_name(package: String, url: &str) -> Option<Address> {
    let client = reqwest::blocking::Client::new();
    let request = format!(r#"{{packageByName(name:"{package}"){{address}}}}"#);

    let res = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "query": request,
            "variables": Value::Null
        }))
        .send()
        .ok()?;
    serde_json::from_value(res.json::<Value>().ok()?["data"]["packageByName"]["address"].clone())
        .ok()
}

fn create_structs(structs: &HashMap<String, MoveStruct>) -> Vec<proc_macro2::TokenStream> {
    structs
        .iter()
        .map(|(name, move_struct)| create_struct(name, move_struct))
        .collect()
}

fn create_struct(struct_name: &str, move_struct: &MoveStruct) -> proc_macro2::TokenStream {
    let (type_parameters, phantoms) = move_struct.type_parameters.iter().enumerate().fold(
        (vec![], vec![]),
        |(mut type_parameters, mut phantoms), (i, v)| {
            let ident = Ident::new(&format!("T{i}"), proc_macro2::Span::call_site());
            type_parameters.push(quote! {#ident});

            if v.is_phantom {
                let name = Ident::new(&format!("phantom_data_{i}"), proc_macro2::Span::call_site());
                let type_: syn::Type =
                    syn::parse_str(&format!("std::marker::PhantomData<T{i}>")).unwrap();
                phantoms.push(quote! {#name: #type_,})
            }
            (type_parameters, phantoms)
        },
    );

    let struct_ident = Ident::new(struct_name, proc_macro2::Span::call_site());
    let field_tokens = move_struct.fields.iter().map(|field| {
        let field_ident = Ident::new(&escape_keyword(&field.name), proc_macro2::Span::call_site());
        let field_type: syn::Type = syn::parse_str(&field.type_.to_rust_type()).unwrap();
        quote! {pub #field_ident: #field_type,}
    });

    let mut derives = vec![
        quote! {serde::Deserialize},
        quote! {serde::Serialize},
        quote! {Debug},
        quote! {MoveStruct},
        quote! {Clone},
    ];

    if move_struct.abilities.has_key() {
        derives.push(quote! {Key});
    }

    if type_parameters.is_empty() {
        quote! {
            #[derive(#(#derives),*)]
            pub struct #struct_ident {
                #(#field_tokens)*
            }
        }
    } else {
        quote! {
            #[derive(#(#derives),*)]
            pub struct #struct_ident<#(#type_parameters),*> {
                #(#field_tokens)*
                #(#phantoms)*
            }
        }
    }
}

fn create_funs(funs: &HashMap<String, MoveFunction>) -> Vec<proc_macro2::TokenStream> {
    funs.iter()
        .flat_map(|(name, fun)| create_fun(name, fun))
        .collect()
}

fn create_fun(fun_name: &str, fun: &MoveFunction) -> Option<proc_macro2::TokenStream> {
    let (param_names, mut params, non_ref_args) = fun.parameters
        .iter()
        .enumerate()
        .fold((vec![], vec![], vec![]), |(mut param_names, mut params, mut non_ref_args), (i, move_type)| {
            let field_ident = Ident::new(&format!("p{i}"), proc_macro2::Span::call_site());
            match &move_type {
                MoveType::Reference(r) |
                MoveType::MutableReference(r) => {
                    // filter out TxContext
                    if matches!(&**r, MoveType::Struct{address, name, ..} if address == &Address::TWO && name.as_str() == "TxContext") {
                        return (param_names, params, non_ref_args);
                    }
                }
                _ => {
                    non_ref_args.push(quote! {#field_ident})
                }
            }
            param_names.push(quote! {#field_ident});
            let field_type: syn::Type = syn::parse_str(&move_type.to_arg_type()).unwrap();
            params.push(quote! {#field_ident: #field_type});
            (param_names, params, non_ref_args)
        });
    params.insert(
        0,
        quote! {builder: &mut sui_transaction_builder::TransactionBuilder},
    );

    let returns = fun
        .return_
        .iter()
        .flat_map(|move_type| syn::parse_str::<syn::Type>(&move_type.to_arg_type()).ok())
        .collect::<Vec<_>>();

    let (types, types_with_ability) = fun.type_parameters.iter().enumerate().fold(
        (vec![], vec![]),
        |(mut types, mut types_with_ability), (i, v)| {
            let ident = Ident::new(&format!("T{i}"), proc_macro2::Span::call_site());
            types.push(ident.clone());
            let mut abilities = vec![quote! {MoveType}, quote! {serde::Serialize}];
            if v.has_key() {
                abilities.push(quote! {move_types::Key});
            }
            types_with_ability.push(quote! {#ident: #(#abilities)+*});
            (types, types_with_ability)
        },
    );

    let fun_ident = Ident::new(fun_name, proc_macro2::Span::call_site());

    let (maybe_returns, maybe_into) = if let Some(t) = returns.first() {
        (quote! { -> #t}, quote! {.into()})
    } else {
        (quote! {}, quote! {;})
    };

    let sig = if types.is_empty() {
        quote! {
            pub fn #fun_ident(#(#params),*) #maybe_returns
        }
    } else {
        quote! {
            pub fn #fun_ident <#(#types_with_ability),*>(#(#params),*) #maybe_returns
        }
    };

    let fun_impl = quote! {
        #sig {
            #(let #non_ref_args = #non_ref_args.maybe_resolve_arg(builder);)*
            builder.move_call(
                sui_transaction_builder::Function::new(
                    PACKAGE_ID,
                    Identifier::from_str(MODULE_NAME).unwrap(),
                    Identifier::from_str(#fun_name).unwrap(),
                    vec![#(#types::type_()),*],
                ),
                vec![#(#param_names.into()),*],
            )
            #maybe_into
        }
    };
    Some(fun_impl)
}

enum SuiNetwork {
    Mainnet,
    Testnet,
}

impl SuiNetwork {
    fn rpc(&self) -> &str {
        match self {
            SuiNetwork::Mainnet => "https://rpc.mainnet.sui.io:443",
            SuiNetwork::Testnet => "https://rpc.testnet.sui.io:443",
        }
    }

    fn gql(&self) -> &str {
        match self {
            SuiNetwork::Mainnet => "https://mvr-rpc.sui-mainnet.mystenlabs.com/graphql",
            SuiNetwork::Testnet => "https://mvr-rpc.sui-testnet.mystenlabs.com/graphql",
        }
    }
}

fn escape_keyword(name: &str) -> String {
    match name {
        "for" | "ref" => {
            format!("{name}_")
        }
        _ => name.to_string(),
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
        address: Address,
        module: Identifier,
        name: Identifier,
        #[serde(default, alias = "typeArguments")]
        type_arguments: Vec<MoveType>,
    },
    Vector(Box<MoveType>),
    Reference(Box<MoveType>),
    MutableReference(Box<MoveType>),
    TypeParameter(u16),
}

impl MoveType {
    fn to_rust_type(&self) -> String {
        match self {
            MoveType::Bool => "bool".to_string(),
            MoveType::U8 => "u8".to_string(),
            MoveType::U16 => "u16".to_string(),
            MoveType::U32 => "u32".to_string(),
            MoveType::U64 => "u64".to_string(),
            MoveType::U128 => "u128".to_string(),
            MoveType::U256 => "move_types::U256".to_string(),
            MoveType::Address => "Address".to_string(),
            MoveType::Signer => "Address".to_string(),
            t @ MoveType::Struct { .. } => t.try_resolve_known_types(),
            MoveType::Vector(t) => {
                format!("Vec<{}>", t.to_rust_type())
            }
            MoveType::Reference(t) => {
                format!("&'static {}", t.to_rust_type())
            }
            MoveType::MutableReference(t) => {
                format!("&'static mut {}", t.to_rust_type())
            }
            MoveType::TypeParameter(index) => format!("T{index}"),
        }
    }

    fn to_arg_type(&self) -> String {
        match self {
            MoveType::Reference(t) => {
                format!("Ref<{}>", t.to_rust_type())
            }
            MoveType::MutableReference(t) => {
                format!("MutRef<{}>", t.to_rust_type())
            }
            _ => format!("Arg<{}>", self.to_rust_type()),
        }
    }

    fn try_resolve_known_types(&self) -> String {
        if let MoveType::Struct {
            address,
            module,
            name,
            type_arguments,
        } = self
        {
            match (address, module.as_str(), name.as_str()) {
                (&MOVE_STDLIB, "type_name", "TypeName") => "String".to_string(),
                (&MOVE_STDLIB, "string", "String") => "String".to_string(),
                (&MOVE_STDLIB, "ascii", "String") => "String".to_string(),
                (&MOVE_STDLIB, "option", "Option") => {
                    format!("Option<{}>", type_arguments[0].to_rust_type())
                }

                (&Address::TWO, "object", "UID") => "sui_sdk_types::ObjectId".to_string(),
                (&Address::TWO, "object", "ID") => "sui_sdk_types::ObjectId".to_string(),
                _ => {
                    if type_arguments.is_empty() {
                        format!("{module}::{name}")
                    } else {
                        format!(
                            "{module}::{name}<{}>",
                            type_arguments.iter().map(|ty| ty.to_rust_type()).join(", ")
                        )
                    }
                }
            }
        } else {
            unreachable!()
        }
    }
}
