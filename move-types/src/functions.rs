use crate::MoveType;
use serde::Serialize;
use sui_sdk_types::Argument;
use sui_transaction_builder::unresolved::Input;
use sui_transaction_builder::{Serialized, TransactionBuilder};

pub enum Arg<T> {
    Resolved(Argument),
    Raw(T),
}

pub enum Ref<'a, T> {
    Resolved(Argument),
    Raw(&'a T),
}

pub enum MutRef<'a, T> {
    Resolved(Argument),
    Raw(&'a mut T),
}

impl<T: MoveType> From<Argument> for MutRef<'_, T> {
    fn from(value: Argument) -> Self {
        Self::Resolved(value)
    }
}

impl<T: MoveType> From<Argument> for Ref<'_, T> {
    fn from(value: Argument) -> Self {
        Self::Resolved(value)
    }
}

impl<T: MoveType> From<Argument> for Arg<T> {
    fn from(value: Argument) -> Self {
        Self::Resolved(value)
    }
}

impl<T: MoveType> From<T> for Arg<T> {
    fn from(value: T) -> Self {
        Self::Raw(value)
    }
}

impl<T> Arg<T> {
    pub fn resolve_arg(self, builder: &mut TransactionBuilder) -> Self
    where
        T: ToInput,
    {
        match self {
            Arg::Raw(value) => Self::Resolved(builder.input(value.to_input())),
            _ => self,
        }
    }
    pub fn borrow(&self) -> Ref<T> {
        match self {
            Arg::Resolved(a) => Ref::Resolved(a.clone()),
            Arg::Raw(p) => Ref::Raw(p),
        }
    }

    pub fn borrow_mut(&mut self) -> MutRef<T> {
        match self {
            Arg::Resolved(a) => MutRef::Resolved(a.clone()),
            Arg::Raw(p) => MutRef::Raw(p),
        }
    }
}

impl<T> Ref<'_, T> {
    pub fn resolve_arg(self, builder: &mut TransactionBuilder) -> Self
    where
        T: ToInput,
    {
        match self {
            Ref::Raw(value) => Self::Resolved(builder.input(value.to_input())),
            _ => self,
        }
    }
}

impl<T> MutRef<'_, T> {
    pub fn resolve_arg(self, builder: &mut TransactionBuilder) -> Self
    where
        T: ToInput,
    {
        match self {
            MutRef::Raw(value) => Self::Resolved(builder.input(value.to_input())),
            _ => self,
        }
    }
}
impl<T> From<Arg<T>> for Argument {
    fn from(value: Arg<T>) -> Self {
        match value {
            Arg::Resolved(arg) => arg,
            Arg::Raw(_) => panic!("Cannot use unresolved arg"),
        }
    }
}
impl<T> From<MutRef<'_, T>> for Argument {
    fn from(value: MutRef<'_, T>) -> Self {
        match value {
            MutRef::Resolved(arg) => arg,
            MutRef::Raw(_) => panic!("Cannot use unresolved arg"),
        }
    }
}
impl<T> From<Ref<'_, T>> for Argument {
    fn from(value: Ref<'_, T>) -> Self {
        match value {
            Ref::Resolved(arg) => arg,
            Ref::Raw(_) => panic!("Cannot use unresolved arg"),
        }
    }
}

pub trait ToInput {
    fn to_input(&self) -> Input;
}

impl<T: MoveType + Serialize> ToInput for T {
    fn to_input(&self) -> Input {
        Serialized(self).into()
    }
}
