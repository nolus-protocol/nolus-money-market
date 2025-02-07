use sdk::schemars::{self, JsonSchema};

use super::super::HigherOrderType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
pub enum ContractsWithoutAdmin {}

impl HigherOrderType for ContractsWithoutAdmin {
    type Of<Unit> = super::ContractsWithoutAdmin<Unit>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
pub enum Contracts {}

impl HigherOrderType for Contracts {
    type Of<Unit> = super::Contracts<Unit>;
}
