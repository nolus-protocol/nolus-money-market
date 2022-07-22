use cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Response, Storage, Timestamp};
use serde::{de::DeserializeOwned, Serialize};

use finance::coin::Coin;
use finance::currency::Currency;
use platform::bank::{self, BankAccount, BankStub};

use crate::error::ContractError;
use crate::lpp::LiquidityPool;
use crate::msg::{QueryLoanOutstandingInterestResponse, QueryLoanResponse, QueryQuoteResponse};

pub fn try_open_loan<LPN>(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Coin<LPN>,
) -> Result<Response, ContractError>
where
    LPN: 'static + Currency + Serialize + DeserializeOwned,
{
    let lease_addr = info.sender;
    let mut lpp = LiquidityPool::<LPN>::load(deps.storage)?;
    lpp.validate_lease_addr(&deps.as_ref(), &lease_addr)?;

    lpp.try_open_loan(&mut deps, &env, lease_addr.clone(), amount)?;

    let bank = BankStub::my_account(&env, &deps.querier);
    let transfer_msg = bank.send(amount, &lease_addr)?;

    let response = Response::new()
        .add_attribute("method", "try_open_loan")
        .add_submessage(transfer_msg);

    Ok(response)
}

pub fn try_repay_loan<LPN>(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError>
where
    LPN: 'static + Currency + Serialize + DeserializeOwned,
{
    let lease_addr = info.sender;
    let repay_amount = bank::received(&info.funds)?;

    let mut lpp = LiquidityPool::<LPN>::load(deps.storage)?;
    lpp.validate_lease_addr(&deps.as_ref(), &lease_addr)?;
    let excess_received = lpp.try_repay_loan(&mut deps, &env, lease_addr.clone(), repay_amount)?;

    let mut response = Response::new().add_attribute("method", "try_repay_loan");

    if !excess_received.is_zero() {
        let bank = BankStub::my_account(&env, &deps.querier);
        let payment = bank.send(excess_received, &lease_addr)?;
        response = response.add_submessage(payment);
    }

    Ok(response)
}

pub fn query_quote<LPN>(
    deps: &Deps,
    env: &Env,
    quote: Coin<LPN>,
) -> Result<QueryQuoteResponse, ContractError>
where
    LPN: 'static + Currency + Serialize + DeserializeOwned,
{
    let lpp = LiquidityPool::<LPN>::load(deps.storage)?;

    match lpp.query_quote(deps, env, quote)? {
        Some(quote) => Ok(QueryQuoteResponse::QuoteInterestRate(quote)),
        None => Ok(QueryQuoteResponse::NoLiquidity),
    }
}

pub fn query_loan<LPN>(
    storage: &dyn Storage,
    env: Env,
    lease_addr: Addr,
) -> Result<QueryLoanResponse<LPN>, ContractError>
where
    LPN: 'static + Currency + Serialize + DeserializeOwned,
{
    LiquidityPool::<LPN>::load(storage)?.query_loan(storage, &env, lease_addr)
}

pub fn query_loan_outstanding_interest<LPN>(
    storage: &dyn Storage,
    loan: Addr,
    outstanding_time: Timestamp,
) -> Result<QueryLoanOutstandingInterestResponse<LPN>, ContractError>
where
    LPN: 'static + Currency + Serialize + DeserializeOwned,
{
    let interest = LiquidityPool::<LPN>::load(storage)?.query_loan_outstanding_interest(
        storage,
        loan,
        outstanding_time,
    )?;

    Ok(interest)
}
