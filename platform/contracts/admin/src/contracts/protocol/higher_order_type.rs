use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use super::super::HigherOrderType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Contracts;

impl HigherOrderType for Contracts {
    type Of<Unit> = super::Contracts<Unit>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Protocol;

impl HigherOrderType for Protocol {
    type Of<Unit> = super::Protocol<Unit>;
}
