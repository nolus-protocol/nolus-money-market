use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use finance::currency::SymbolOwned;
use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};

use crate::common::{
    type_defs::{MaybeMigrateGeneralContract, MigrateGeneralContracts, MigrateLpnContracts},
    GeneralContracts, LpnContracts,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InstantiateMsg {
    pub general_contracts: GeneralContracts<Addr>,
    pub lpn_contracts: HashMap<SymbolOwned, LpnContracts<Addr>>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SudoMsg {
    RegisterLpnContracts {
        symbol: SymbolOwned,
        contracts: LpnContracts<Addr>,
    },
    MigrateContracts(Box<MigrateContracts>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MigrateContracts {
    pub release: String,
    pub admin_contract: MaybeMigrateGeneralContract,
    pub general_contracts: MigrateGeneralContracts,
    pub lpn_contracts: MigrateLpnContracts,
}
