use access_control::SingleUserAccess;
use currency::native::Nls;
use finance::coin::Coin;
use platform::{
    bank::{self, BankAccount},
    batch::Batch,
    response::{self},
};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Storage},
};
use versioning::{version, VersionSegment};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, SudoMsg},
    result::ContractResult,
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
) -> ContractResult<CwResponse> {
    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    Ok(response::empty_response())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> ContractResult<CwResponse> {
    versioning::update_software(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    SingleUserAccess::remove_contract_owner(deps.storage);

    response::response(versioning::release())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    let sender = info.sender;
    match msg {
        ExecuteMsg::SendRewards { amount } => {
            let mut bank_account = bank::account(&env.contract.address, &deps.querier);

            try_send_rewards(deps.storage, sender, amount, &mut bank_account)?;
            let batch: Batch = bank_account.into();
            Ok(response::response_only_messages(batch))
        }
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
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
) -> ContractResult<CwResponse> {
    SingleUserAccess::new(
        crate::access_control::REWARDS_DISPATCHER_NAMESPACE,
        rewards_dispatcher,
    )
    .store(storage)?;

    Ok(response::empty_response())
}

fn try_send_rewards<B>(
    storage: &dyn Storage,
    sender: Addr,
    amount: Coin<Nls>,
    account: &mut B,
) -> ContractResult<()>
where
    B: BankAccount,
{
    SingleUserAccess::load(storage, crate::access_control::REWARDS_DISPATCHER_NAMESPACE)?
        .check_access(&sender)?;

    account.send(amount, &sender);

    Ok(())
}
