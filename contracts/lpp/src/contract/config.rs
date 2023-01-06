use access_control::SingleUserAccess;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Deps, DepsMut, MessageInfo, StdResult},
};

use crate::{borrow::InterestRate, error::ContractError, state::Config};

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
