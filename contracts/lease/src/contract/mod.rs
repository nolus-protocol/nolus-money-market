#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cw_storage_plus::Item,
    {
        cosmwasm_ext::Response as CwResponse,
        cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply},
    },
};

use crate::{
    contract::state::{Response, State},
    error::{ContractError, ContractResult},
};

use self::{state::Controller, msg::{NewLeaseForm, ExecuteMsg, StateQuery}};

mod alarms;
mod close;
mod cmd;
// mod dex;
pub mod msg;
mod repay;
mod state;

const DB_ITEM: Item<State> = Item::new("state");

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    form: NewLeaseForm,
) -> ContractResult<CwResponse> {
    load_mut(&deps)?
        .instantiate(&mut deps, env, info, form)
        .and_then(
            |Response {
                 cw_response,
                 next_state,
             }| {
                save(&next_state, &mut deps)?;

                Ok(cw_response)
            },
        )
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn reply(mut deps: DepsMut, env: Env, msg: Reply) -> ContractResult<CwResponse> {
    load_mut(&deps)?.reply(&mut deps, env, msg).and_then(
        |Response {
             cw_response,
             next_state,
         }| {
            save(&next_state, &mut deps)?;

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
    load_mut(&deps)?
        .execute(&mut deps, env, info, msg)
        .and_then(
            |Response {
                 cw_response,
                 next_state,
             }| {
                save(&next_state, &mut deps)?;

                Ok(cw_response)
            },
        )
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps, env: Env, msg: StateQuery) -> ContractResult<Binary> {
    load(&deps)?.query(deps, env, msg)
}

fn load(deps: &Deps) -> ContractResult<State> {
    Ok(DB_ITEM.may_load(deps.storage)?.unwrap_or_default())
}

fn load_mut(deps: &DepsMut) -> ContractResult<State> {
    load(&deps.as_ref())
}
fn save(next_state: &State, deps: &mut DepsMut) -> ContractResult<()> {
    DB_ITEM
        .save(deps.storage, next_state)
        .map_err(ContractError::from)
}
