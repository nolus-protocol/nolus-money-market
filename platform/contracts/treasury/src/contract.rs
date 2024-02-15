use access_control::SingleUserAccess;
use currency::NlsPlatform;
use finance::coin::Coin;
use platform::{
    bank::{self, BankAccount},
    message::Response as PlatformResponse,
    response,
};
use sdk::{
    cosmwasm_ext::{as_dyn::storage, Response as CwResponse},
    cosmwasm_std::{entry_point, Addr, DepsMut, Env, MessageInfo},
};
use versioning::{package_version, version, SemVer, Version, VersionSegment};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, SudoMsg},
    result::ContractResult,
};

// version info for migration info
// const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 0;
const PACKAGE_VERSION: SemVer = package_version!();
const CONTRACT_VERSION: Version = version!(CONTRACT_STORAGE_VERSION, PACKAGE_VERSION);

#[entry_point]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<CwResponse> {
    versioning::initialize(deps.storage, CONTRACT_VERSION)?;

    try_configure_reward_dispatcher(deps.storage, &msg.rewards_dispatcher)?;

    Ok(response::empty_response())
}

#[entry_point]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> ContractResult<CwResponse> {
    versioning::update_software(deps.storage, CONTRACT_VERSION, Into::into)
        .and_then(response::response)
}

#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    match msg {
        ExecuteMsg::SendRewards { amount } => {
            let mut bank_account = bank::account(&env.contract.address, deps.querier);

            try_send_rewards(deps.storage, info.sender, amount, &mut bank_account)?;

            Ok(response::response_only_messages(
                PlatformResponse::messages_only(bank_account.into()),
            ))
        }
    }
}

#[entry_point]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    match msg {
        SudoMsg::ConfigureRewardTransfer { rewards_dispatcher } => {
            platform::contract::validate_addr(deps.querier, &rewards_dispatcher)?;

            try_configure_reward_dispatcher(deps.storage, &rewards_dispatcher)?;

            Ok(response::empty_response())
        }
    }
}

fn try_configure_reward_dispatcher<S>(
    storage: &mut S,
    rewards_dispatcher: &Addr,
) -> ContractResult<()>
where
    S: storage::DynMut + ?Sized,
{
    SingleUserAccess::new(storage, crate::access_control::REWARDS_DISPATCHER_NAMESPACE)
        .grant_to(rewards_dispatcher)
        .map_err(Into::into)
}

fn try_send_rewards<S, B>(
    storage: &S,
    sender: Addr,
    amount: Coin<NlsPlatform>,
    account: &mut B,
) -> ContractResult<()>
where
    S: storage::Dyn + ?Sized,
    B: BankAccount,
{
    SingleUserAccess::new(storage, crate::access_control::REWARDS_DISPATCHER_NAMESPACE)
        .check(&sender)?;

    account.send(amount, &sender);

    Ok(())
}
