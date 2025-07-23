use std::collections::BTreeMap;

use platform::contract::Validator;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::error::Error;

pub(crate) trait Validate
where
    Self: Sized,
{
    type Context<'r>: Copy;

    type Error;

    fn validate(&self, ctx: Self::Context<'_>) -> Result<(), Self::Error>;
}

pub(crate) trait Map {
    type Key;

    type Value;

    type Values<'r>: Iterator<Item = &'r Self::Value>
    where
        Self: 'r,
        Self::Value: 'r;

    fn values(&self) -> Self::Values<'_>;
}

impl<K, V> Map for BTreeMap<K, V> {
    type Key = K;

    type Value = V;

    type Values<'r>
        = std::collections::btree_map::Values<'r, K, V>
    where
        Self::Key: 'r,
        Self::Value: 'r;

    fn values(&self) -> Self::Values<'_> {
        self.values()
    }
}

pub(crate) struct ValidateValues<'r, M>(&'r M)
where
    M: Map,
    M::Value: Validate;

impl<'r, M> ValidateValues<'r, M>
where
    M: Map,
    M::Value: Validate,
{
    pub const fn new(map: &'r M) -> Self {
        Self(map)
    }
}

impl<M> Validate for ValidateValues<'_, M>
where
    M: Map,
    M::Value: Validate,
{
    type Context<'t> = <M::Value as Validate>::Context<'t>;

    type Error = <M::Value as Validate>::Error;

    fn validate(&self, ctx: Self::Context<'_>) -> Result<(), Self::Error> {
        self.0.values().try_for_each(|value| value.validate(ctx))
    }
}

impl Validate for Addr {
    type Context<'r> = QuerierWrapper<'r>;

    type Error = Error;

    fn validate(&self, ctx: Self::Context<'_>) -> Result<(), Self::Error> {
        platform::contract::validator(ctx)
            .check_contract(self)
            .map_err(Error::Platform)
    }
}
