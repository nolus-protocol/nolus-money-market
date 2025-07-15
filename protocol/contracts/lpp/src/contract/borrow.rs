use currency::{Currency, CurrencyDef};
use finance::coin::Coin;
use platform::{
    bank::{self, BankAccount},
    batch::Batch,
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Storage};

use crate::{
    loans::Repo,
    lpp::LiquidityPool,
    msg::{LoanResponse, QueryLoanResponse, QueryQuoteResponse},
};

use super::Result;

pub(super) fn try_open_loan<Lpn>(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    amount: Coin<Lpn>,
) -> Result<(LoanResponse<Lpn>, MessageResponse)>
where
    Lpn: 'static + CurrencyDef,
{
    let lease_addr = info.sender;
    let mut lpp = LiquidityPool::<Lpn>::load(deps.storage)?;

    let loan = lpp.try_open_loan(&mut deps, &env, lease_addr.clone(), amount)?;

    let mut bank = bank::account(&env.contract.address, deps.querier);
    bank.send(amount, lease_addr);

    let messages: Batch = bank.into();

    Ok((loan, messages.into()))
}

pub(super) fn try_repay_loan<Lpn>(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<(Coin<Lpn>, MessageResponse)>
where
    Lpn: CurrencyDef,
{
    let lease_addr = info.sender;
    let repay_amount = bank::received_one(&info.funds)?;

    let mut lpp = LiquidityPool::<Lpn>::load(deps.storage)?;
    let excess_received = lpp.try_repay_loan(&mut deps, &env, lease_addr.clone(), repay_amount)?;

    let batch = if excess_received.is_zero() {
        Batch::default()
    } else {
        let mut bank = bank::account(&env.contract.address, deps.querier);
        bank.send(excess_received, lease_addr);
        bank.into()
    };
    Ok((excess_received, batch.into()))
}

pub(super) fn query_quote<Lpn>(
    deps: &Deps<'_>,
    env: &Env,
    quote: Coin<Lpn>,
) -> Result<QueryQuoteResponse>
where
    Lpn: CurrencyDef,
{
    let lpp = LiquidityPool::<Lpn>::load(deps.storage)?;

    match lpp.query_quote(quote, &env.contract.address, deps.querier, &env.block.time)? {
        Some(quote) => Ok(QueryQuoteResponse::QuoteInterestRate(quote)),
        None => Ok(QueryQuoteResponse::NoLiquidity),
    }
}

pub fn query_loan<Lpn>(storage: &dyn Storage, lease_addr: Addr) -> Result<QueryLoanResponse<Lpn>>
where
    Lpn: 'static + Currency,
{
    Repo::query(storage, lease_addr)
}

pub fn query_empty<Lpn>(storage: &dyn Storage) -> bool {
    Repo::<Lpn>::empty(storage)
}
