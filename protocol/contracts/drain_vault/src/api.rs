use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::Addr;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct InstantiateMsg {
    /// Unchecked address of the owner allowed to drive the sweep — the profit contract.
    pub owner: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Send the full NLS balance the vault holds to the recipient. Owner-gated.
    Sweep { recipient: Addr },
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    /// Return a [ConfigResponse]
    Config(),
    /// Implementation of [versioning::query::ProtocolPackage::Release]
    ProtocolPackageRelease {},
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
pub struct ConfigResponse {
    pub owner: Addr,
}
