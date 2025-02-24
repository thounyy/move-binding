pub use move_core_types::account_address::AccountAddress;
pub use move_core_types::ident_str;
pub use move_core_types::identifier::IdentStr;
pub use move_core_types::identifier::Identifier;
pub use move_core_types::language_storage::StructTag;
pub use move_core_types::language_storage::TypeTag;

pub trait MoveType {
    fn type_() -> TypeTag;
}

pub trait MoveStruct {
    fn struct_type() -> StructTag;
}

impl<T: MoveStruct> MoveType for T {
    fn type_() -> TypeTag {
        TypeTag::Struct(Self::struct_type().into())
    }
}

impl MoveType for u64 {
    fn type_() -> TypeTag {
        TypeTag::U64
    }
}
