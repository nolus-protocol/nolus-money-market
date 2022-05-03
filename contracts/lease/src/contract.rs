use cosmwasm_std::{entry_point};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::opening::NewLeaseForm;
use crate::error::ContractError;
use crate::lease::Lease;
use crate::msg::{ExecuteMsg, QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: NewLeaseForm,
) -> Result<Response, ContractError> {
    // TODO restrict the Lease instantiation only to the Leaser addr by using `nolusd tx wasm store ... --instantiate-only-address <addr>`
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let lease: Lease = msg.into(deps.api)?;
    // TODO validate "SingleDenom" invariant
    lease.store(deps.storage)?;

    // TODO query lpp about its denom
    // const LPP_DENOM: &str = "UST";

    // let borrow_amount = Coin::new(amount, denom)
    // lpp::msg::ExecuteMsg::Loan{}
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[entry_point]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    // match msg {
    // QueryMsg::Config {} => to_binary(&query_config(deps)?),
    // }
    StdResult::Ok(Binary::from([]))
}
