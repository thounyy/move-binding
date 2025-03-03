use crate::MoveType;
use serde::Serialize;
use std::marker::PhantomData;
use sui_sdk_types::Argument;
use sui_transaction_builder::{Serialized, TransactionBuilder};

pub struct Arg<T: MoveType> {
    inner: Option<Argument>,
    data: Option<T>,
}

pub struct Ref<T: MoveType> {
    phantom_data: PhantomData<T>,
    inner: Argument,
}

pub struct MutRef<T: MoveType> {
    phantom_data: PhantomData<T>,
    inner: Argument,
}

impl<T: MoveType> From<Argument> for MutRef<T> {
    fn from(value: Argument) -> Self {
        Self {
            phantom_data: Default::default(),
            inner: value,
        }
    }
}

impl<T: MoveType> From<Argument> for Ref<T> {
    fn from(value: Argument) -> Self {
        Self {
            phantom_data: Default::default(),
            inner: value,
        }
    }
}

impl<T: MoveType> From<Argument> for Arg<T> {
    fn from(value: Argument) -> Self {
        Self {
            inner: Some(value),
            data: None,
        }
    }
}

impl<T: MoveType> From<Arg<T>> for Argument {
    fn from(value: Arg<T>) -> Self {
        value.inner.unwrap()
    }
}
impl<T: MoveType> From<MutRef<T>> for Argument {
    fn from(value: MutRef<T>) -> Self {
        value.inner
    }
}
impl<T: MoveType> From<Ref<T>> for Argument {
    fn from(value: Ref<T>) -> Self {
        value.inner
    }
}

impl<T: MoveType> From<T> for Arg<T> {
    fn from(value: T) -> Self {
        Self {
            // dummy input, need to be resolved later
            inner: None,
            data: Some(value),
        }
    }
}

impl<T: MoveType + Serialize> Arg<T> {
    pub fn maybe_resolve_arg(self, builder: &mut TransactionBuilder) -> Self {
        if self.inner.is_none() {
            if let Some(data) = &self.data {
                return Self {
                    inner: Some(builder.input(Serialized(data))),
                    data: None,
                };
            }
        }
        self
    }

    pub fn borrow(&self) -> Ref<T> {
        Ref {
            phantom_data: Default::default(),
            inner: self.inner.expect("Cannot borrow unresolved Arg."),
        }
    }

    pub fn borrow_mut(&mut self) -> MutRef<T> {
        MutRef {
            phantom_data: Default::default(),
            inner: self.inner.expect("Cannot borrow unresolved Arg."),
        }
    }
}
