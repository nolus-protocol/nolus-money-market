use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use finance::currency::SymbolOwned;
use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};

use crate::common::{
    type_defs::{MaybeMigrateGeneral, MigrateGeneralContracts, MigrateSpecializedContracts},
    GeneralContractsGroup, SpecializedContractsGroup,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InstantiateMsg {
    pub general_contracts: GeneralContractsGroup<Addr>,
    pub specialized_contracts: HashMap<SymbolOwned, SpecializedContractsGroup<Addr>>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SudoMsg {
    AddSpecializedGroup {
        symbol: SymbolOwned,
        specialized_contracts: SpecializedContractsGroup<Addr>,
    },
    Migrate(Box<MigrateContracts>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MigrateContracts {
    pub release: String,
    pub admin_contract: MaybeMigrateGeneral,
    pub general_contracts: MigrateGeneralContracts,
    pub specialized_contracts: MigrateSpecializedContracts,
}
