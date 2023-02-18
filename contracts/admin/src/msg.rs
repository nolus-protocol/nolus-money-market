use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use finance::currency::SymbolOwned;
use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};

use crate::common::{GeneralContractsGroup, SpecializedContractsGroup};

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
pub struct CodeIdWithMigrateMsg<M> {
    pub code_id: u64,
    pub migrate_msg: M,
}

pub type ContractsMigrateIndividual = CodeIdWithMigrateMsg<String>;

pub type GeneralContractsMigrateIndividual = CodeIdWithMigrateMsg<String>;
pub type GeneralContractsMaybeMigrateIndividual = Option<GeneralContractsMigrateIndividual>;
pub type GeneralContractsMaybeMigrate =
    GeneralContractsGroup<GeneralContractsMaybeMigrateIndividual>;

pub type SpecializedContractsMigrateIndividual = CodeIdWithMigrateMsg<HashMap<SymbolOwned, String>>;
pub type SpecializedContractsMaybeMigrateIndividual = Option<SpecializedContractsMigrateIndividual>;
pub type SpecializedContractsMaybeMigrate =
    SpecializedContractsGroup<SpecializedContractsMaybeMigrateIndividual>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ExecuteMsg {
    #[cfg(any(debug_assertions, test, feature = "admin_contract_exec"))]
    LocalNetSudo { sudo: SudoMsg },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MigrateContractsData {
    pub release: String,
    pub admin_contract: GeneralContractsMaybeMigrateIndividual,
    pub general_contracts: GeneralContractsMaybeMigrate,
    pub specialized_contracts: SpecializedContractsMaybeMigrate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SudoMsg {
    RegisterSymbolGroup {
        symbol: SymbolOwned,
        specialized_contracts: SpecializedContractsGroup<Addr>,
    },
    Migrate(Box<MigrateContractsData>),
}
