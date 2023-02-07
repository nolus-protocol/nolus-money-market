use serde::{de::DeserializeOwned, Serialize};

use finance::{coin::Coin, currency::Currency};
use platform::{
    bank::{self, BankAccount},
    batch::Batch,
};
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{to_binary, Addr, Deps, DepsMut, Env, MessageInfo, Storage, Timestamp},
};

use crate::{
    error::ContractError,
    lpp::LiquidityPool,
    msg::{
        LoanResponse, QueryLoanOutstandingInterestResponse, QueryLoanResponse, QueryQuoteResponse,
    },
};

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

    let annual_interest_rate = lpp.try_open_loan(&mut deps, &env, lease_addr.clone(), amount)?;

    let mut bank = bank::account(&env.contract.address, &deps.querier);
    bank.send(amount, &lease_addr);

    let loan_response = LoanResponse {
        principal_due: amount,
        interest_due: Coin::new(0),
        annual_interest_rate,
        interest_paid: env.block.time,
    };

    let batch: Batch = bank.into();

    let mut response: Response = batch.into();

    response = response.add_attribute("method", "try_open_loan");

    response = response.set_data(to_binary(&loan_response)?);

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
    let repay_amount = bank::received_one(info.funds)?;

    let mut lpp = LiquidityPool::<LPN>::load(deps.storage)?;
    lpp.validate_lease_addr(&deps.as_ref(), &lease_addr)?;
    let excess_received = lpp.try_repay_loan(&mut deps, &env, lease_addr.clone(), repay_amount)?;

    let batch = if excess_received.is_zero() {
        Batch::default()
    } else {
        let mut bank = bank::account(&env.contract.address, &deps.querier);
        bank.send(excess_received, &lease_addr);
        bank.into()
    };

    let mut resp: Response = batch.into();
    resp = resp
        .add_attribute("method", "try_repay_loan")
        .set_data(to_binary(&excess_received)?);
    Ok(resp)
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

    match lpp.query_quote(quote, &env.contract.address, &deps.querier, env.block.time)? {
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
