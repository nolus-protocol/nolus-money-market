use sdk::schemars::{self, JsonSchema};

pub trait HigherOrderType {
    type Of<T>;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, JsonSchema)]
pub struct Identity;

impl HigherOrderType for Identity {
    type Of<T> = T;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, JsonSchema)]
pub struct Option;

impl HigherOrderType for Option {
    type Of<T> = core::option::Option<T>;
}
