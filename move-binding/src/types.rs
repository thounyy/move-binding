use itertools::Itertools;
use move_binary_format::normalized::Type;
use move_core_types::account_address::AccountAddress;

pub trait ToRustType {
    fn to_rust_type(&self) -> String;
    fn is_ref(&self) -> bool;
    fn to_arg_type(&self) -> String;
    fn try_resolve_known_types(&self) -> String;
}

impl ToRustType for Type {
    fn to_rust_type(&self) -> String {
        match self {
            Self::Bool => "bool".to_string(),
            Self::U8 => "u8".to_string(),
            Self::U16 => "u16".to_string(),
            Self::U32 => "u32".to_string(),
            Self::U64 => "u64".to_string(),
            Self::U128 => "u128".to_string(),
            Self::U256 => "move_types::U256".to_string(),
            Self::Address => "Address".to_string(),
            Self::Signer => "Address".to_string(),
            t @ Self::Struct { .. } => t.try_resolve_known_types(),
            Self::Vector(t) => {
                format!("Vec<{}>", t.to_rust_type())
            }
            Self::Reference(t) => {
                format!("&'static {}", t.to_rust_type())
            }
            Self::MutableReference(t) => {
                format!("&'static mut {}", t.to_rust_type())
            }
            Self::TypeParameter(index) => format!("T{index}"),
        }
    }

    fn is_ref(&self) -> bool {
        match self {
            Self::Reference(_) | Self::MutableReference(_) => true,
            _ => false,
        }
    }

    fn to_arg_type(&self) -> String {
        match self {
            Self::Reference(t) => {
                format!("Ref<'a, {}>", t.to_rust_type())
            }
            Self::MutableReference(t) => {
                format!("MutRef<'a, {}>", t.to_rust_type())
            }
            _ => format!("Arg<{}>", self.to_rust_type()),
        }
    }

    fn try_resolve_known_types(&self) -> String {
        if let Self::Struct {
            address,
            module,
            name,
            type_arguments,
        } = self
        {
            match (address, module.as_str(), name.as_str()) {
                (&AccountAddress::ONE, "type_name", "TypeName") => "String".to_string(),
                (&AccountAddress::ONE, "string", "String") => "String".to_string(),
                (&AccountAddress::ONE, "ascii", "String") => "String".to_string(),
                (&AccountAddress::ONE, "option", "Option") => {
                    format!("Option<{}>", type_arguments[0].to_rust_type())
                }

                (&AccountAddress::TWO, "object", "UID") => "sui_sdk_types::ObjectId".to_string(),
                (&AccountAddress::TWO, "object", "ID") => "sui_sdk_types::ObjectId".to_string(),
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
