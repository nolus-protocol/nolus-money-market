use cosmwasm_std::{Addr, BankMsg, Coin as CwCoin, Deps, DepsMut, Env, Response, Storage};
use finance::currency::{Currency, Nls};
use serde::{de::DeserializeOwned, Serialize};

use crate::error::ContractError;
use crate::lpp::LiquidityPool;
use crate::msg::{LppBalanceResponse, RewardsResponse};
use crate::state::Deposit;
use finance::coin_legacy;

pub fn try_distribute_rewards(
    deps: DepsMut,
    funds: Vec<CwCoin>,
) -> Result<Response, ContractError> {
    match funds.iter().find(|&coin| coin.denom == Nls::SYMBOL) {
        Some(coin) => Deposit::distribute_rewards(deps, coin.to_owned())?,
        None => {
            return Err(ContractError::CustomError {
                val: "Rewards are supported only in native currency".to_string(),
            })
        }
    }

    Ok(Response::new().add_attribute("method", "try_distribute_rewards"))
}

pub fn try_claim_rewards(
    deps: DepsMut,
    addr: Addr,
    other_recipient: Option<Addr>,
) -> Result<Response, ContractError> {
    let recipient = other_recipient.unwrap_or_else(|| addr.clone());
    let mut deposit = Deposit::load(deps.storage, addr)?;
    let reward = deposit.claim_rewards(deps.storage)?;

    let msg = BankMsg::Send {
        to_address: recipient.into(),
        amount: vec![reward],
    };

    let response = Response::new()
        .add_attribute("method", "try_claim_rewards")
        .add_message(msg);

    Ok(response)
}

pub fn query_lpp_balance<LPN>(
    deps: Deps,
    env: Env,
) -> Result<LppBalanceResponse<LPN>, ContractError>
where
    LPN: Currency + DeserializeOwned + Serialize,
{
    let lpp = LiquidityPool::<LPN>::load(deps.storage)?;
    Ok(lpp.query_lpp_balance(&deps, &env)?)
}

pub fn query_rewards(storage: &dyn Storage, addr: Addr) -> Result<RewardsResponse, ContractError> {
    let deposit = Deposit::load(storage, addr)?;
    let rewards = deposit.query_rewards(storage)?;
    Ok(RewardsResponse {
        rewards: coin_legacy::from_cosmwasm(rewards)?,
    })
}
