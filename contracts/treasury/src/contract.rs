use access_control::SingleUserAccess;
use currency::native::Nls;
use finance::coin::Coin;
use platform::{
    bank::{self, BankAccount},
    batch::Batch,
    response,
};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, StdResult, Storage},
};
use versioning::{version, VersionSegment};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, SudoMsg},
};

// version info for migration info
// const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 0;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    Ok(Response::default())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    versioning::update_software(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    SingleUserAccess::remove_contract_owner(deps.storage);

    response::response(versioning::release()).map(Into::into)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let sender = info.sender;
    match msg {
        ExecuteMsg::SendRewards { amount } => {
            let mut bank_account = bank::account(&env.contract.address, &deps.querier);

            try_send_rewards(deps.storage, sender, amount, &mut bank_account)?;
            let batch: Batch = bank_account.into();
            let mut response: Response = batch.into();
            response = response.add_attribute("method", "try_send_rewards");
            Ok(response)
        }
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::ConfigureRewardTransfer { rewards_dispatcher } => {
            platform::contract::validate_addr(&deps.querier, &rewards_dispatcher)?;

            try_configure_reward_transfer(deps.storage, rewards_dispatcher)
        }
    }
}

fn try_configure_reward_transfer(
    storage: &mut dyn Storage,
    rewards_dispatcher: Addr,
) -> Result<Response, ContractError> {
    SingleUserAccess::new(
        crate::access_control::REWARDS_DISPATCHER_NAMESPACE,
        rewards_dispatcher,
    )
    .store(storage)?;

    Ok(Response::new().add_attribute("method", "try_configure_reward_transfer"))
}

fn try_send_rewards<B>(
    storage: &dyn Storage,
    sender: Addr,
    amount: Coin<Nls>,
    account: &mut B,
) -> Result<(), ContractError>
where
    B: BankAccount,
{
    SingleUserAccess::load(storage, crate::access_control::REWARDS_DISPATCHER_NAMESPACE)?
        .check_access(&sender)?;

    account.send(amount, &sender);

    Ok(())
}
