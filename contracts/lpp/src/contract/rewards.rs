use cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Response, Storage};
use platform::batch::Batch;
use serde::{de::DeserializeOwned, Serialize};

use crate::error::ContractError;
use crate::lpp::LiquidityPool;
use crate::msg::{LppBalanceResponse, RewardsResponse};
use crate::state::Deposit;
use finance::coin::Coin;
use finance::currency::{Currency, Nls};
use platform::bank::{self, BankAccount, BankStub};

pub fn try_distribute_rewards(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let amount: Coin<Nls> = bank::received(&info.funds)?;
    Deposit::distribute_rewards(deps, amount)?;

    Ok(Response::new().add_attribute("method", "try_distribute_rewards"))
}

pub fn try_claim_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    other_recipient: Option<Addr>,
) -> Result<Response, ContractError> {
    let recipient = other_recipient.unwrap_or_else(|| info.sender.clone());
    let mut deposit = Deposit::load(deps.storage, info.sender)?;
    let reward = deposit.claim_rewards(deps.storage)?;

    let mut bank = BankStub::my_account(&env, &deps.querier);
    bank.send(reward, &recipient);

    let batch: Batch = bank.into();

    let mut batch: Response = batch.into();
    batch = batch.add_attribute("method", "try_claim_rewards");
    Ok(batch)
}

pub fn query_lpp_balance<LPN>(
    deps: Deps,
    env: Env,
) -> Result<LppBalanceResponse<LPN>, ContractError>
where
    LPN: 'static + Currency + DeserializeOwned + Serialize,
{
    let lpp = LiquidityPool::<LPN>::load(deps.storage)?;
    lpp.query_lpp_balance(&deps, &env)
}

pub fn query_rewards(storage: &dyn Storage, addr: Addr) -> Result<RewardsResponse, ContractError> {
    let deposit = Deposit::load(storage, addr)?;
    let rewards = deposit.query_rewards(storage)?;
    Ok(RewardsResponse { rewards })
}
