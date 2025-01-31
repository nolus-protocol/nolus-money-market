use std::marker::PhantomData;

use sdk::schemars::{self, JsonSchema};

pub trait FirstOrderType<DerivedFrom>
where
    Self: Sized,
    DerivedFrom: HigherOrderType<Of<Self::Unit> = Self>,
{
    type Unit;
}

pub trait HigherOrderType
where
    Self: Sized,
{
    type Of<T>: FirstOrderType<Self, Unit = T>;
}

impl<T> FirstOrderType<Identity> for T {
    type Unit = Self;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, JsonSchema)]
pub enum Identity {}

impl HigherOrderType for Identity {
    type Of<T> = T;
}

impl<Left, Right> FirstOrderType<HigherOrderTuple<false, Left>> for (Left, Right) {
    type Unit = Right;
}

impl<Left, Right> FirstOrderType<HigherOrderTuple<true, Right>> for (Left, Right) {
    type Unit = Left;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
pub struct HigherOrderTuple<const BOUND_RIGHT: bool, Bound> {
    _bound: PhantomData<Bound>,
}

impl<Left> HigherOrderType for HigherOrderTuple<false, Left> {
    type Of<Right> = (Left, Right);
}

impl<Right> HigherOrderType for HigherOrderTuple<true, Right> {
    type Of<Left> = (Left, Right);
}

impl<T> FirstOrderType<Option> for core::option::Option<T> {
    type Unit = T;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, JsonSchema)]
pub enum Option {}

impl HigherOrderType for Option {
    type Of<T> = core::option::Option<T>;
}

impl<T, Outer, Inner> FirstOrderType<Compose<Outer, Inner>> for T
where
    Self: FirstOrderType<Outer, Unit: FirstOrderType<Inner>>,
    Outer: HigherOrderType<Of<<Self as FirstOrderType<Outer>>::Unit> = Self>,
    Inner: HigherOrderType<
        Of<<<Self as FirstOrderType<Outer>>::Unit as FirstOrderType<Inner>>::Unit> = <Self as FirstOrderType<Outer>>::Unit,
    >,
{
    type Unit = <<Self as FirstOrderType<Outer>>::Unit as FirstOrderType<Inner>>::Unit;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
pub struct Compose<Outer, Inner> {
    _outer: PhantomData<Outer>,
    _inner: PhantomData<Inner>,
}

impl<Outer, Inner> HigherOrderType for Compose<Outer, Inner>
where
    Outer: HigherOrderType,
    Inner: HigherOrderType,
{
    type Of<T> = Outer::Of<Inner::Of<T>>;
}
