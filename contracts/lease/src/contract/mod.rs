mod close;
mod open;
mod repay;
mod state;

#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response};
use cw2::set_contract_version;
use platform::bank::BankStub;
use platform::batch::Batch;

use crate::error::ContractResult;
use crate::lease::{self, LeaseDTO};
use crate::msg::{ExecuteMsg, NewLeaseForm, StateQuery};

use self::close::Close;
use self::open::{OpenLoanReq, OpenLoanResp};
use self::repay::Repay;
use self::state::LeaseState;

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    form: NewLeaseForm,
) -> ContractResult<Response> {
    // TODO restrict the Lease instantiation only to the Leaser addr by using `nolusd tx wasm store ... --instantiate-only-address <addr>`
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let lease = form.into_lease_dto(env.block.time, deps.api, &deps.querier)?;
    lease.store(deps.storage)?;

    let batch = lease::execute(lease, OpenLoanReq::new(&info.funds), &deps.querier)?;

    Ok(batch.into())
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> ContractResult<Response> {
    // TODO swap the received loan and the downpayment to lease.currency
    let lease = LeaseDTO::load(deps.storage)?;

    let batch = lease::execute(lease, OpenLoanResp::new(msg), &deps.querier)?;

    Ok(batch.into())
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    let lease = LeaseDTO::load(deps.storage)?;

    let batch = match msg {
        ExecuteMsg::Repay() => try_repay(deps, env, info, lease),
        ExecuteMsg::Close() => try_close(deps, env, info, lease),
    }?;
    Ok(batch.into())
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn query(deps: Deps, env: Env, _msg: StateQuery) -> ContractResult<Binary> {
    let lease = LeaseDTO::load(deps.storage)?;

    let bank = BankStub::my_account(&env, &deps.querier);

    // TODO think on taking benefit from having a LppView trait
    lease::execute(
        lease,
        LeaseState::new(env.block.time, bank, env.contract.address.clone()),
        &deps.querier,
    )
}

fn try_repay(deps: DepsMut, env: Env, info: MessageInfo, lease: LeaseDTO) -> ContractResult<Batch> {
    lease::execute(
        lease,
        Repay::new(&info.funds, env.block.time, env.contract.address),
        &deps.querier,
    )
}

fn try_close(deps: DepsMut, env: Env, info: MessageInfo, lease: LeaseDTO) -> ContractResult<Batch> {
    let bank = BankStub::my_account(&env, &deps.querier);

    let batch = lease::execute(
        lease,
        Close::new(&info.sender, env.contract.address.clone(), bank),
        &deps.querier,
    )?;
    Ok(batch)
}
