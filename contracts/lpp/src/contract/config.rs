use cosmwasm_std::{Deps, DepsMut, Response};

use crate::error::ContractError;
use crate::msg::QueryConfigResponse;
use crate::state::Config;
use finance::percent::Percent;

pub fn try_update_parameters(
    deps: DepsMut,
    base_interest_rate: Percent,
    utilization_optimal: Percent,
    addon_optimal_interest_rate: Percent,
) -> Result<Response, ContractError> {
    Config::load(deps.storage)?.update(
        deps.storage,
        base_interest_rate,
        utilization_optimal,
        addon_optimal_interest_rate,
    )?;
    Ok(Response::new().add_attribute("method", "try_update_parameters"))
}

pub fn query_config(deps: &Deps) -> Result<QueryConfigResponse, ContractError> {
    let config = Config::load(deps.storage)?;

    Ok(QueryConfigResponse {
        lpn_symbol: config.currency,
        lease_code_id: config.lease_code_id,
        base_interest_rate: config.base_interest_rate,
        utilization_optimal: config.utilization_optimal,
        addon_optimal_interest_rate: config.addon_optimal_interest_rate,
    })
}
