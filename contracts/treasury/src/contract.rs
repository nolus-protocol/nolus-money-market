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
    cw2::set_contract_version,
};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg},
    state::{self, ADMIN, REWARDS_DISPATCHER},
};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = info.sender;
    ADMIN.save(deps.storage, &admin)?;

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
            try_configure_reward_transfer(deps, sender, rewards_dispatcher)
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
    deps: DepsMut,
    sender: Addr,
    rewards_dispatcher: Addr,
) -> Result<Response, ContractError> {
    state::assert_admin(deps.storage, sender)?;
    deps.api.addr_validate(rewards_dispatcher.as_str())?;
    REWARDS_DISPATCHER.save(deps.storage, &rewards_dispatcher)?;
    Ok(Response::new().add_attribute("method", "try_configure_reward_transfer"))
}

fn try_send_rewards<B>(
    storage: &mut dyn Storage,
    sender: Addr,
    amount: Coin<Nls>,
    mut account: B,
) -> Result<B, ContractError>
where
    B: BankAccount,
{
    state::assert_rewards_dispatcher(storage, &sender)?;
    account.send(amount, &sender);

    Ok(account)
}
