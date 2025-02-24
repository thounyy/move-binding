use sui_types::TypeTag;

pub trait MoveType {
    fn type_() -> TypeTag;
}

impl MoveType for u64 {
    fn type_() -> TypeTag {
        TypeTag::U64
    }
}
