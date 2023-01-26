use access_control::SingleUserAccess;
use cosmwasm_std::Uint64;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Deps, DepsMut, MessageInfo, StdResult},
};

use crate::{borrow::InterestRate, error::ContractError, state::Config};

pub fn try_update_lease_code(
    deps: DepsMut,
    info: MessageInfo,
    lease_code: Uint64,
) -> Result<Response, ContractError> {
    SingleUserAccess::load(deps.storage, crate::access_control::LEASE_CODE_ADMIN_KEY)?
        .check_access(&info.sender)?;

    Config::update_lease_code(deps.storage, lease_code)?;

    Ok(Response::new().add_attribute("method", "try_update_lease_code"))
}

pub fn try_update_parameters(
    deps: DepsMut,
    info: MessageInfo,
    interest_rate: InterestRate,
) -> Result<Response, ContractError> {
    SingleUserAccess::check_owner_access::<ContractError>(deps.storage, &info.sender)?;

    Config::update_borrow_rate(deps.storage, interest_rate)?;

    Ok(Response::new().add_attribute("method", "try_update_parameters"))
}

pub fn query_config(deps: &Deps) -> StdResult<Config> {
    Config::load(deps.storage)
}
