use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};
use versioning::ReleaseId;

use crate::state::{reward_scale::RewardScale, CadenceHours};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub cadence_hours: CadenceHours,
    pub protocols_registry: Addr,
    pub timealarms: Addr,
    pub tvl_to_apr: RewardScale,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MigrateMsg {
    pub to_release: ReleaseId,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    TimeAlarm {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum SudoMsg {
    Config { cadence_hours: CadenceHours },
    Rewards { tvl_to_apr: RewardScale },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    CalculateRewards {},
    /// Implementation of [versioning::query::PlatformPackage::Release]
    PlatformPackageRelease {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ConfigResponse {
    pub cadence_hours: CadenceHours,
}

pub type RewardScaleResponse = RewardScale;

#[cfg(test)]
mod test {

    use platform::tests as platform_tests;

    use super::QueryMsg;

    #[test]
    fn release() {
        assert_eq!(
            Ok(QueryMsg::PlatformPackageRelease {}),
            platform_tests::ser_de(&versioning::query::PlatformPackage::Release {}),
        );
    }
}
