#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Api, DepsMut, Env, MessageInfo, Reply, Response, StdResult};
use cw2::set_contract_version;
use cw_utils::one_coin;
use lpp::stub::{Lpp, LppStub};

use crate::error::{ContractResult, ContractError};
use crate::msg::{NewLeaseForm, ExecuteMsg};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: NewLeaseForm,
) -> ContractResult<Response> {
    // TODO restrict the Lease instantiation only to the Leaser addr by using `nolusd tx wasm store ... --instantiate-only-address <addr>`
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let downpayment = one_coin(&info)?;
    let borrow = msg.amount_to_borrow(&downpayment)?;
    let lpp = lpp(msg.loan.lpp.clone(), deps.api)?;
    msg.save(deps.storage)?;
    let req = lpp.open_loan_req(borrow)?;

    // TODO define an OpenLoanRequest(downpayment, borrowed_amount) and persist it

    Ok(Response::new().add_submessage(req))
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> ContractResult<Response> {
    // TODO debug_assert the balance is increased with the borrowed amount
    // TODO load the top request and pass it as a reply
    let new_lease_form = NewLeaseForm::pull(deps.storage)?;
    let lpp = lpp(new_lease_form.loan.lpp.clone(), deps.api)?;
    lpp.open_loan_resp(msg).map_err(ContractError::OpenLoanError)?;

    let lease = new_lease_form.into_lease(lpp, deps.api)?;
    lease.store(deps.storage)?;

    Ok(Response::default())
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> ContractResult<Response> {
    Ok(Response::default())
}

// #[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
// pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
//     // match msg {
//     // QueryMsg::Config {} => to_binary(&query_config(deps)?),
//     // }
//     StdResult::Ok(Binary::from([]))
// }

fn lpp(address: String, api: &dyn Api) -> StdResult<LppStub> {
    lpp::stub::LppStub::try_from(address, api)
}
