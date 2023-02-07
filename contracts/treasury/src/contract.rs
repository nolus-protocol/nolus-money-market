use access_control::SingleUserAccess;
use currency::native::Nls;
use finance::coin::Coin;
use platform::{
    bank::{self, BankAccount},
    batch::Batch,
};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Storage},
};
use versioning::{package_version, Version};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg},
};

const CONTRACT_STORAGE_VERSION_FROM: Version = 0;
const CONTRACT_STORAGE_VERSION: Version = 0;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    versioning::initialize::<CONTRACT_STORAGE_VERSION>(deps.storage, package_version!())?;

    SingleUserAccess::new_contract_owner(info.sender).store(deps.storage)?;

    Ok(Response::default())
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct MigrateMsg {}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    versioning::upgrade_old_contract::<0, CONTRACT_STORAGE_VERSION_FROM, CONTRACT_STORAGE_VERSION>(
        deps.storage,
        package_version!(),
    )?;

    sdk::cw_storage_plus::Item::<String>::new("contract_info").remove(deps.storage);

    Ok(Response::default())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let sender = info.sender;
    match msg {
        ExecuteMsg::ConfigureRewardTransfer { rewards_dispatcher } => {
            platform::contract::validate_addr(&deps.querier, &rewards_dispatcher)?;

            try_configure_reward_transfer(deps.storage, sender, rewards_dispatcher)
        }
        ExecuteMsg::SendRewards { amount } => {
            let bank_account = bank::my_account(&env, &deps.querier);

            let bank_account = try_send_rewards(deps.storage, sender, amount, bank_account)?;
            let batch: Batch = bank_account.into();
            let mut response: Response = batch.into();
            response = response.add_attribute("method", "try_send_rewards");
            Ok(response)
        }
    }
}

fn try_configure_reward_transfer(
    storage: &mut dyn Storage,
    sender: Addr,
    rewards_dispatcher: Addr,
) -> Result<Response, ContractError> {
    SingleUserAccess::check_owner_access::<ContractError>(storage, &sender)?;

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
    mut account: B,
) -> Result<B, ContractError>
where
    B: BankAccount,
{
    SingleUserAccess::load(storage, crate::access_control::REWARDS_DISPATCHER_NAMESPACE)?
        .check_access(&sender)?;

    account.send(amount, &sender);

    Ok(account)
}
