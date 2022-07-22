use cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Response, Storage, Uint128};
use serde::{de::DeserializeOwned, Serialize};

use finance::currency::Currency;
use finance::coin::Coin;
use finance::bank::{self, BankStub, BankAccount};

use crate::error::ContractError;
use crate::lpp::LiquidityPool;
use crate::msg::{BalanceResponse, PriceResponse};
use crate::state::Deposit;

pub fn try_deposit<LPN>(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError>
where
    LPN: 'static + Currency + DeserializeOwned + Serialize,
{
    let lender_addr = info.sender;
    let amount = bank::received(&info.funds)?;

    let lpp = LiquidityPool::<LPN>::load(deps.storage)?;

    let price = lpp.calculate_price(&deps.as_ref(), &env)?;
    Deposit::load(deps.storage, lender_addr)?.deposit(deps.storage, amount, price)?;

    Ok(Response::new().add_attribute("method", "try_deposit"))
}

pub fn try_withdraw<LPN>(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount_nlpn: Uint128,
) -> Result<Response, ContractError>
where
    LPN: 'static + Currency + DeserializeOwned + Serialize,
{
    let lender_addr = info.sender;
    let amount_nlpn = Coin::new(amount_nlpn.u128());

    let lpp = LiquidityPool::<LPN>::load(deps.storage)?;
    let payment_lpn = lpp.withdraw_lpn(&deps.as_ref(), &env, amount_nlpn)?;

    let maybe_reward =
        Deposit::load(deps.storage, lender_addr.clone())?.withdraw(deps.storage, amount_nlpn)?;

    let mut response = Response::new().add_attribute("method", "try_withdraw");

    let bank = BankStub::my_account(&env, &deps.querier);
    let payment_msg = bank.send(payment_lpn, &lender_addr)?;
    response = response.add_submessage(payment_msg);

    if let Some(reward) = maybe_reward {
        let reward_msg = bank.send(reward, &lender_addr)?;
        response = response.add_submessage(reward_msg);
    }

    Ok(response)
}

pub fn query_ntoken_price<LPN>(deps: Deps, env: Env) -> Result<PriceResponse<LPN>, ContractError>
where
    LPN: Currency + DeserializeOwned + Serialize,
{
    let lpp = LiquidityPool::<LPN>::load(deps.storage)?;
    let price = lpp.calculate_price(&deps, &env)?.into();

    Ok(price)
}

pub fn query_balance(storage: &dyn Storage, addr: Addr) -> Result<BalanceResponse, ContractError> {
    let balance: u128 = Deposit::query_balance_nlpn(storage, addr)?
        .unwrap_or_default()
        .into();
    Ok(BalanceResponse {
        balance: balance.into(),
    })
}
