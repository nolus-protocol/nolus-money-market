use cosmwasm_std::{Addr, BankMsg, Coin as CwCoin, Deps, DepsMut, Env, Response, Storage, Uint128};
use serde::{de::DeserializeOwned, Serialize};

use crate::error::ContractError;
use crate::lpp::{IntoCW, LiquidityPool};
use crate::msg::{BalanceResponse, PriceResponse};
use crate::state::Deposit;
use finance::currency::Currency;

pub fn try_deposit<LPN>(
    deps: DepsMut,
    env: Env,
    lender_addr: Addr,
    funds: Vec<CwCoin>,
) -> Result<Response, ContractError>
where
    LPN: Currency + DeserializeOwned + Serialize,
{
    if funds.len() != 1 {
        return Err(ContractError::FundsLen {});
    }

    let lpp = LiquidityPool::<LPN>::load(deps.storage)?;
    let amount = lpp.try_into_amount(funds[0].clone())?;

    let price = lpp.calculate_price(&deps.as_ref(), &env)?;
    let amount: u128 = amount.into();
    Deposit::load(deps.storage, lender_addr)?.deposit(deps.storage, amount.into(), price)?;

    Ok(Response::new().add_attribute("method", "try_deposit"))
}

pub fn try_withdraw<LPN>(
    deps: DepsMut,
    env: Env,
    lender_addr: Addr,
    amount_nlpn: Uint128,
) -> Result<Response, ContractError>
where
    LPN: Currency + DeserializeOwned + Serialize,
{
    let lpp = LiquidityPool::<LPN>::load(deps.storage)?;

    let payment_lpn = lpp.withdraw_lpn(&deps.as_ref(), &env, amount_nlpn)?;
    let mut msg_payment = vec![payment_lpn.into_cw()];

    let maybe_reward =
        Deposit::load(deps.storage, lender_addr.clone())?.withdraw(deps.storage, amount_nlpn)?;

    if let Some(reward_msg) = maybe_reward {
        msg_payment.push(reward_msg)
    }

    let msg = BankMsg::Send {
        to_address: lender_addr.into(),
        amount: msg_payment,
    };

    let response = Response::new()
        .add_attribute("method", "try_withdraw")
        .add_message(msg);

    Ok(response)
}

pub fn query_ntoken_price<LPN>(deps: Deps, env: Env) -> Result<PriceResponse, ContractError>
where
    LPN: Currency + DeserializeOwned + Serialize,
{
    let lpp = LiquidityPool::<LPN>::load(deps.storage)?;
    let price = lpp.calculate_price(&deps, &env)?.into();

    Ok(price)
}

pub fn query_balance(storage: &dyn Storage, addr: Addr) -> Result<BalanceResponse, ContractError> {
    let balance = Deposit::query_balance_nlpn(storage, addr)?.unwrap_or_default();
    Ok(BalanceResponse { balance })
}
