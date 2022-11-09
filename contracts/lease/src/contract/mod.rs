#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply},
    neutron_sdk::sudo::msg::SudoMsg,
};

use crate::{
    api::{ExecuteMsg, NewLeaseForm, StateQuery},
    contract::state::Controller,
};
use crate::{contract::state::Response, error::ContractResult};

mod alarms;
mod close;
mod cmd;
pub mod msg;
mod repay;
mod state;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    form: NewLeaseForm,
) -> ContractResult<CwResponse> {
    impl_::load_mut(&deps)?
        .instantiate(&mut deps, env, info, form)
        .and_then(
            |Response {
                 cw_response,
                 next_state,
             }| {
                impl_::save(&next_state, &mut deps)?;

                Ok(cw_response)
            },
        )
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn reply(mut deps: DepsMut, env: Env, msg: Reply) -> ContractResult<CwResponse> {
    impl_::load_mut(&deps)?.reply(&mut deps, env, msg).and_then(
        |Response {
             cw_response,
             next_state,
         }| {
            impl_::save(&next_state, &mut deps)?;

            Ok(cw_response)
        },
    )
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    impl_::load_mut(&deps)?
        .execute(&mut deps, env, info, msg)
        .and_then(
            |Response {
                 cw_response,
                 next_state,
             }| {
                impl_::save(&next_state, &mut deps)?;

                Ok(cw_response)
            },
        )
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(mut deps: DepsMut, env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    impl_::load_mut(&deps)?.sudo(&mut deps, env, msg).and_then(
        |Response {
             cw_response,
             next_state,
         }| {
            impl_::save(&next_state, &mut deps)?;

            Ok(cw_response)
        },
    )
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps, env: Env, msg: StateQuery) -> ContractResult<Binary> {
    impl_::load(&deps)?.query(deps, env, msg)
}

mod impl_ {
    use cosmwasm_std::{Deps, DepsMut};
    use sdk::cw_storage_plus::Item;

    use crate::error::{ContractError, ContractResult};

    use super::state::State;

    const STATE_DB_KEY: Item<State> = Item::new("state");

    pub(super) fn load(deps: &Deps) -> ContractResult<State> {
        Ok(STATE_DB_KEY.may_load(deps.storage)?.unwrap_or_default())
    }

    pub(super) fn load_mut(deps: &DepsMut) -> ContractResult<State> {
        load(&deps.as_ref())
    }
    pub(super) fn save(next_state: &State, deps: &mut DepsMut) -> ContractResult<()> {
        STATE_DB_KEY
            .save(deps.storage, next_state)
            .map_err(ContractError::from)
    }
}
