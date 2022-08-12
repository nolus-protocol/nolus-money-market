mod close;
mod open;
mod repay;
mod state;

#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Reply, Response,
};
use cw2::set_contract_version;
use platform::{
    bank::BankStub,
    batch::{Emit, Emitter},
};

use crate::error::ContractResult;
use crate::lease::{self, LeaseDTO};
use crate::msg::{ExecuteMsg, NewLeaseForm, StateQuery};

use self::open::{OpenLoanReq, OpenLoanResp};
use self::repay::Repay;
use self::state::LeaseState;
use self::{close::Close, repay::RepayResult};

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

    let emitter = lease::execute(lease, OpenLoanReq::new(&info.funds), &deps.querier)?;

    Ok(emitter.into())
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

    match msg {
        ExecuteMsg::Repay() => {
            let res = try_repay(&deps.querier, env, info, lease)?;
            LeaseDTO::store(&res.lease_dto, deps.storage)?;
            Ok(res.emitter)
        }
        ExecuteMsg::Close() => try_close(deps, env, info, lease),
    }
    .map(Into::into)
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

fn try_repay(
    querier: &QuerierWrapper,
    env: Env,
    info: MessageInfo,
    lease: LeaseDTO,
) -> ContractResult<RepayResult> {
    lease::execute(
        lease,
        Repay::new(&info.funds, env.block.time, env.contract.address.clone()),
        querier,
    )
    .map(|mut result| {
        result.emitter = result
            .emitter
            .emit_tx_info(&env)
            .emit("to", env.contract.address);

        result
    })
}

fn try_close(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lease: LeaseDTO,
) -> ContractResult<Emitter> {
    let bank = BankStub::my_account(&env, &deps.querier);

    let emitter = lease::execute(
        lease,
        Close::new(
            &info.sender,
            env.contract.address.clone(),
            bank,
            env.block.time,
        ),
        &deps.querier,
    )?;
    Ok(emitter)
}
