use ::currency::lease::LeaseGroup;
use cosmwasm_std::to_binary;
use finance::currency;
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply},
    cw_storage_plus::Item,
    neutron_sdk::sudo::msg::SudoMsg,
};
use versioning::Version;

use crate::{
    api::{ExecuteMsg, MigrateMsg, NewLeaseForm, StateQuery},
    contract::{state::Controller, state::Response},
    error::{ContractError, ContractResult},
};

use self::state::RequestLoan;

mod alarms;
mod cmd;
pub mod msg;
mod repay;
mod state;

const CONTRACT_VERSION: Version = 0;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    form: NewLeaseForm,
) -> ContractResult<CwResponse> {
    currency::validate::<LeaseGroup>(&form.currency)?;
    deps.api.addr_validate(form.customer.as_str())?;

    platform::contract::validate_addr(&deps.querier, &form.time_alarms)?;
    platform::contract::validate_addr(&deps.querier, &form.market_price_oracle)?;
    platform::contract::validate_addr(&deps.querier, &form.loan.lpp)?;
    platform::contract::validate_addr(&deps.querier, &form.loan.profit)?;

    versioning::initialize::<CONTRACT_VERSION>(deps.storage)?;

    let (batch, next_state) = RequestLoan::new(&mut deps, info, form)?;
    impl_::save(&next_state.into(), &mut deps)?;
    Ok(batch.into())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> ContractResult<CwResponse> {
    // the version is 0 so the previos code was deployed in the previos epoch
    versioning::initialize::<CONTRACT_VERSION>(deps.storage)?;
    Item::<bool>::new("contract_info").remove(deps.storage);

    Ok(CwResponse::default())
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
    let resp = impl_::load(&deps)?.query(deps, env, msg)?;
    to_binary(&resp).map_err(ContractError::from)
}

mod impl_ {
    use sdk::{
        cosmwasm_std::{Deps, DepsMut},
        cw_storage_plus::Item,
    };

    use crate::error::{ContractError, ContractResult};

    use super::state::State;

    const STATE_DB_KEY: Item<State> = Item::new("state");

    pub(super) fn load(deps: &Deps) -> ContractResult<State> {
        Ok(STATE_DB_KEY.load(deps.storage)?)
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
