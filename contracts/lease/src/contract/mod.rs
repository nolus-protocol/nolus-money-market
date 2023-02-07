use ::currency::lease::LeaseGroup;
use cosmwasm_std::to_binary;
use finance::currency;
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply},
    neutron_sdk::sudo::msg::SudoMsg,
};
use serde::{Deserialize, Serialize};
use versioning::{package_version, Version};

use crate::{
    api::{ExecuteMsg, MigrateMsg, NewLeaseContract, StateQuery},
    contract::{state::Controller, state::Response},
    dex::Account,
    error::{ContractError, ContractResult},
    lease::LeaseDTO,
};

use self::state::{v0::Migrate, RequestLoan};

mod cmd;
pub mod msg;
mod state;

const EXPECTED_MIGRATION_STORAGE_VERSION: Version = 0;
const CONTRACT_STORAGE_VERSION: Version = 0;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
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

    versioning::initialize::<CONTRACT_STORAGE_VERSION>(deps.storage, package_version!())?;

    let (batch, next_state) = RequestLoan::new(&mut deps, info, new_lease)?;
    impl_::save(&next_state.into(), &mut deps)?;
    Ok(batch.into())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(mut deps: DepsMut, env: Env, _msg: MigrateMsg) -> ContractResult<CwResponse> {
    versioning::upgrade_old_contract::<
        1,
        EXPECTED_MIGRATION_STORAGE_VERSION,
        CONTRACT_STORAGE_VERSION,
    >(deps.storage, package_version!())?;

    sdk::cw_storage_plus::Item::<String>::new("contract_info").remove(deps.storage);

    {
        let migrated_contract = impl_::load_v0(&deps)?.into_last_version(env.contract.address);
        impl_::save(&migrated_contract, &mut deps)?;
    }

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

#[derive(Serialize, Deserialize)]
pub(crate) struct Lease {
    pub lease: LeaseDTO,
    pub dex: Account,
}

mod impl_ {
    use sdk::{
        cosmwasm_std::{Deps, DepsMut},
        cw_storage_plus::Item,
    };

    use crate::error::{ContractError, ContractResult};

    use super::state::v0::StateV0;
    use super::state::State;

    const STATE_DB_KEY: &str = "state";
    const STATE_DB_ITEM: Item<State> = Item::new(STATE_DB_KEY);

    pub(super) fn load(deps: &Deps) -> ContractResult<State> {
        Ok(STATE_DB_ITEM.load(deps.storage)?)
    }

    pub(super) fn load_mut(deps: &DepsMut) -> ContractResult<State> {
        load(&deps.as_ref())
    }

    pub(super) fn load_v0(deps: &DepsMut) -> ContractResult<StateV0> {
        Ok(Item::new(STATE_DB_KEY).load(deps.storage)?)
    }

    pub(super) fn save(next_state: &State, deps: &mut DepsMut) -> ContractResult<()> {
        STATE_DB_ITEM
            .save(deps.storage, next_state)
            .map_err(ContractError::from)
    }
}
