use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{Addr, QuerierWrapper},
    schemars::{self, JsonSchema},
};

use crate::{
    common::{
        type_defs::{
            MaybeMigrateGeneralContract, MigrateGeneralContracts, MigrateLpnContracts,
            ValidateAddresses as _,
        },
        GeneralContracts, LpnContracts,
    },
    error::ContractError,
};

// use crate::{
//     common::{type_defs::MigrateContract, GeneralContracts, LpnContracts, ValidateAddresses as _},
//     error::ContractError,
// };

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InstantiateMsg {
    pub general_contracts: GeneralContracts<Addr>,
    pub lpn_contracts: LpnContracts<Addr>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SudoMsg {
    MigrateContracts(MigrateContracts),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MigrateContracts {
    pub release: String,
    pub admin_contract: MaybeMigrateGeneralContract,
    pub general_contracts: MigrateGeneralContracts,
    pub lpn_contracts: MigrateLpnContracts,
}

impl InstantiateMsg {
    pub(crate) fn validate(&self, querier: &QuerierWrapper<'_>) -> Result<(), ContractError> {
        self.general_contracts.validate(querier)?;

        self.lpn_contracts.validate(querier)
    }
}
