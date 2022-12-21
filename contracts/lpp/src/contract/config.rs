use access_control::SingleUserAccess;
use finance::percent::Percent;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Deps, DepsMut, MessageInfo},
};

use crate::{borrow::InterestRate, error::ContractError, msg::QueryConfigResponse, state::Config};

pub fn try_update_parameters(
    deps: DepsMut,
    info: MessageInfo,
    interest_rate: InterestRate,
) -> Result<Response, ContractError> {
    SingleUserAccess::check_owner_access::<ContractError>(deps.storage, &info.sender)?;

    Config::update_borrow_rate(deps.storage, interest_rate)?;

    Ok(Response::new().add_attribute("method", "try_update_parameters"))
}

pub fn query_config(deps: &Deps) -> Result<QueryConfigResponse, ContractError> {
    let config = Config::load(deps.storage)?;

    let interest_rate = config.borrow_rate();

    Ok(QueryConfigResponse {
        lpn_ticker: config.lpn_ticker().into(),
        lease_code_id: config.lease_code_id(),
        base_interest_rate: interest_rate.base_interest_rate(),
        utilization_optimal: interest_rate.utilization_optimal(),
        addon_optimal_interest_rate: interest_rate.addon_optimal_interest_rate(),
    })
}
