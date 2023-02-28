use enum_dispatch::enum_dispatch;

use ::currency::lease::LeaseGroup;
use finance::currency;
use platform::batch::Batch;
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{to_binary, Api, Binary, Deps, DepsMut, Env, MessageInfo, Reply},
    neutron_sdk::sudo::msg::SudoMsg,
};
use versioning::{respond_with_release, version, VersionSegment};

use crate::{
    api::{ExecuteMsg, MigrateMsg, NewLeaseContract, StateQuery},
    contract::Contract,
    error::{ContractError, ContractResult},
};

use super::{opening::request_loan::RequestLoan, Response};

// const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 1;

#[enum_dispatch]
pub(crate) trait Controller
where
    Self: Sized,
{
    fn enter(&self, deps: Deps<'_>, _env: Env) -> ContractResult<Batch> {
        err("enter", deps.api)
    }

    fn reply(self, deps: &mut DepsMut<'_>, _env: Env, _msg: Reply) -> ContractResult<Response> {
        err("reply", deps.api)
    }

    fn execute(
        self,
        deps: &mut DepsMut<'_>,
        _env: Env,
        _info: MessageInfo,
        _msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        err("execute", deps.api)
    }

    fn on_open_ica(
        self,
        _counterparty_version: String,
        deps: Deps<'_>,
        _env: Env,
    ) -> ContractResult<Response> {
        err("sudo open ica", deps.api)
    }

    fn on_response(self, _data: Binary, deps: Deps<'_>, _env: Env) -> ContractResult<Response> {
        err("sudo response", deps.api)
    }

    fn on_error(self, deps: Deps<'_>, _env: Env) -> ContractResult<Response> {
        err("sudo error", deps.api)
    }

    fn on_timeout(self, deps: Deps<'_>, _env: Env) -> ContractResult<Response> {
        err("sudo timeout", deps.api)
    }
}

pub(super) fn err<R>(op: &str, api: &dyn Api) -> ContractResult<R> {
    let err = ContractError::unsupported_operation(op);
    api.debug(&format!("{:?}", op));

    Err(err)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    new_lease: NewLeaseContract,
) -> ContractResult<CwResponse> {
    //TODO move the following validation into the deserialization
    currency::validate::<LeaseGroup>(&new_lease.form.currency)?;
    deps.api.addr_validate(new_lease.form.customer.as_str())?;

    platform::contract::validate_addr(&deps.querier, &new_lease.form.time_alarms)?;
    platform::contract::validate_addr(&deps.querier, &new_lease.form.market_price_oracle)?;
    platform::contract::validate_addr(&deps.querier, &new_lease.form.loan.lpp)?;
    platform::contract::validate_addr(&deps.querier, &new_lease.form.loan.profit)?;

    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    let (batch, next_state) = RequestLoan::new(&mut deps, info, new_lease)?;
    super::save(deps.storage, &next_state.into())?;
    Ok(batch.into())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> ContractResult<CwResponse> {
    versioning::update_software(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    respond_with_release().map_err(Into::into)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn reply(mut deps: DepsMut<'_>, env: Env, msg: Reply) -> ContractResult<CwResponse> {
    super::load(deps.storage)?
        .reply(&mut deps, env, msg)
        .and_then(
            |Response {
                 cw_response,
                 next_state,
             }| {
                super::save(deps.storage, &next_state)?;

                Ok(cw_response)
            },
        )
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    super::load(deps.storage)?
        .execute(&mut deps, env, info, msg)
        .and_then(
            |Response {
                 cw_response,
                 next_state,
             }| {
                super::save(deps.storage, &next_state)?;

                Ok(cw_response)
            },
        )
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    let state = super::load(deps.storage)?;
    match msg {
        SudoMsg::OpenAck {
            port_id: _,
            channel_id: _,
            counterparty_channel_id: _,
            counterparty_version,
        } => state.on_open_ica(counterparty_version, deps.as_ref(), env),
        SudoMsg::Response { request: _, data } => state.on_response(data, deps.as_ref(), env),
        SudoMsg::Timeout { request: _ } => state.on_timeout(deps.as_ref(), env),
        SudoMsg::Error {
            request: _,
            details: _,
        } => state.on_error(deps.as_ref(), env),
        _ => unreachable!(),
    }
    .and_then(
        |Response {
             cw_response,
             next_state,
         }| {
            super::save(deps.storage, &next_state)?;

            Ok(cw_response)
        },
    )
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps<'_>, env: Env, _msg: StateQuery) -> ContractResult<Binary> {
    let resp = super::load(deps.storage)?.state(env.block.time, &deps.querier)?;
    to_binary(&resp).map_err(ContractError::from)
}
