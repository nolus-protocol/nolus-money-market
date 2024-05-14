use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use super::super::HigherOrderType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProtocolContracts;

impl HigherOrderType for ProtocolContracts {
    type Of<T> = super::ProtocolContracts<T>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Protocol;

impl HigherOrderType for Protocol {
    type Of<T> = super::Protocol<T>;
}
