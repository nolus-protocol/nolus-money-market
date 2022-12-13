use currency::native::Nls;
use finance::coin::Coin;
use platform::{
    bank::{self, BankAccount},
    batch::Batch,
};
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{entry_point, Addr, Deps, DepsMut, Env, MessageInfo},
    cw2::set_contract_version,
};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg},
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

    crate::access_control::OWNER.set_address(deps, info.sender)?;

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

            let bank_account = try_send_rewards(deps.as_ref(), sender, amount, bank_account)?;
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
    crate::access_control::OWNER.assert_address::<_, ContractError>(deps.as_ref(), &sender)?;

    deps.api.addr_validate(rewards_dispatcher.as_str())?;

    crate::access_control::REWARDS_DISPATCHER.set_address(deps, rewards_dispatcher)?;

    Ok(Response::new().add_attribute("method", "try_configure_reward_transfer"))
}

fn try_send_rewards<B>(
    deps: Deps,
    sender: Addr,
    amount: Coin<Nls>,
    mut account: B,
) -> Result<B, ContractError>
where
    B: BankAccount,
{
    crate::access_control::REWARDS_DISPATCHER.assert_address::<_, ContractError>(deps, &sender)?;

    account.send(amount, &sender);

    Ok(account)
}
