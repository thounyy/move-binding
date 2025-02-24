pub use move_core_types::account_address::AccountAddress;
pub use move_core_types::ident_str;
pub use move_core_types::identifier::IdentStr;
pub use move_core_types::language_storage::StructTag;
pub use move_core_types::language_storage::TypeTag;
pub use move_core_types::identifier::Identifier;

pub trait MoveType {
    fn type_() -> TypeTag;
}

impl MoveType for u64 {
    fn type_() -> TypeTag {
        TypeTag::U64
    }
}
