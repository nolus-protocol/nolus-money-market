use crate::{bindings::query::NeutronQuery, NeutronError, NeutronResult};
use cosmwasm_std::{Coin, Deps, StdError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TotalBurnedNeutronsAmountResponse {
    pub coin: Coin,
}

/// Returns total amount of burned neutron fees
pub fn query_total_burned_neutrons(
    deps: Deps<NeutronQuery>,
) -> NeutronResult<TotalBurnedNeutronsAmountResponse> {
    let query = NeutronQuery::TotalBurnedNeutronsAmount {};
    Ok(deps
        .querier
        .query(&query.into())
        .map_err(|error: StdError| NeutronError::Std(error.to_string()))?)
}
