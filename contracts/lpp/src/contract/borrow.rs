use cosmwasm_std::{
    Addr, BankMsg, Coin as CwCoin, Deps, DepsMut, Env, Response, Storage, Timestamp,
};
use serde::{de::DeserializeOwned, Serialize};

use finance::coin::Coin;
use finance::currency::Currency;

use crate::error::ContractError;
use crate::lpp::{IntoCW, LiquidityPool};
use crate::msg::{QueryLoanOutstandingInterestResponse, QueryLoanResponse, QueryQuoteResponse};

pub fn try_open_loan<LPN>(
    deps: DepsMut,
    env: Env,
    lease_addr: Addr,
    amount: Coin<LPN>,
) -> Result<Response, ContractError>
where
    LPN: 'static + Currency + Serialize + DeserializeOwned,
{
    let mut lpp = LiquidityPool::<LPN>::load(deps.storage)?;
    lpp.validate_lease_addr(&deps.as_ref(), &lease_addr)?;

    lpp.try_open_loan(deps, env, lease_addr.clone(), amount)?;

    // TODO: transition to finance bank module
    let transfer_msg = BankMsg::Send {
        to_address: lease_addr.to_string(),
        amount: vec![amount.into_cw()],
    };

    let response = Response::new()
        .add_attribute("method", "try_open_loan")
        .add_message(transfer_msg);

    Ok(response)
}

pub fn try_repay_loan<LPN>(
    deps: DepsMut,
    env: Env,
    lease_addr: Addr,
    funds: Vec<CwCoin>,
) -> Result<Response, ContractError>
where
    LPN: 'static + Currency + Serialize + DeserializeOwned,
{
    if funds.len() != 1 {
        return Err(ContractError::FundsLen {});
    }

    let repay_amount = funds[0].clone();
    let repay_amount = Coin::new(repay_amount.amount.u128());

    let mut lpp = LiquidityPool::<LPN>::load(deps.storage)?;
    lpp.validate_lease_addr(&deps.as_ref(), &lease_addr)?;
    let excess_received = lpp.try_repay_loan(deps, env, lease_addr.clone(), repay_amount)?;

    let mut response = Response::new().add_attribute("method", "try_repay_loan");

    if excess_received != Coin::new(0) {
        // TODO: transition to finance bank module
        let payment = lpp.pay(lease_addr, excess_received);
        response = response.add_message(payment);
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
