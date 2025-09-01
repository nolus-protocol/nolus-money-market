use crate::{
    bindings::{msg::IbcFee, query::NeutronQuery},
    NeutronError, NeutronResult,
};
use cosmwasm_std::{Deps, StdError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MinIbcFeeResponse {
    pub min_fee: IbcFee,
}

pub fn query_min_ibc_fee(deps: Deps<NeutronQuery>) -> NeutronResult<MinIbcFeeResponse> {
    let query = NeutronQuery::MinIbcFee {};
    Ok(deps
        .querier
        .query(&query.into())
        .map_err(|error: StdError| NeutronError::Std(error.to_string()))?)
}
