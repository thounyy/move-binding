use crate::MoveType;
use std::marker::PhantomData;
use sui_sdk_types::Argument;

pub struct Arg<T: MoveType> {
    phantom_data: PhantomData<T>,
    inner: Argument,
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
            phantom_data: Default::default(),
            inner: value,
        }
    }
}

impl<T: MoveType> From<Arg<T>> for Argument {
    fn from(value: Arg<T>) -> Self {
        value.inner
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
