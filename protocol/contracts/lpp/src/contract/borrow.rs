use currency::{Currency, CurrencyDef};
use finance::coin::Coin;
use platform::{
    bank::{self, BankAccount},
    batch::Batch,
    contract,
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Storage};

use crate::{
    contract::ContractError,
    loans::Repo,
    lpp::LiquidityPool,
    msg::{LoanResponse, QueryLoanResponse, QueryQuoteResponse},
    state::Config,
};

use super::Result;

pub(super) fn try_open_loan<Lpn>(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    amount: Coin<Lpn>,
) -> Result<(LoanResponse<Lpn>, MessageResponse)>
where
    Lpn: 'static + CurrencyDef,
{
    let config = Config::load(deps.storage)?;
    let mut bank = bank::account(&env.contract.address, deps.querier);
    let mut lpp = LiquidityPool::load(deps.storage, &config, &bank)?;
    let lease_addr = lpp.validate_lease_addr(&contract::validator(deps.querier), info.sender)?;

    let loan = lpp.try_open_loan(env.block.time, amount)?;
    Repo::open(deps.storage, lease_addr.clone(), &loan)?;

    lpp.save(deps.storage)?;

    bank.send(amount, lease_addr);

    let messages: Batch = bank.into();

    Ok((loan, messages.into()))
}

pub(super) fn try_repay_loan<Lpn>(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<(Coin<Lpn>, MessageResponse)>
where
    Lpn: CurrencyDef,
{
    let repay_amount = bank::received_one(&info.funds)?;

    let config = Config::load(deps.storage)?;
    let mut bank = bank::account(&env.contract.address, deps.querier);
    let mut lpp = LiquidityPool::load(deps.storage, &config, &bank)?;
    let lease_addr = lpp.validate_lease_addr(&contract::validator(deps.querier), info.sender)?;

    let payment = Repo::load(deps.storage, lease_addr.clone()).and_then(|mut loan| {
        let now = env.block.time;
        let payment = loan.repay(&now, repay_amount).ok_or_else(|| {
            ContractError::overflow_loan_repayment("Trying to repay loan", now, repay_amount)
        })?;
        Repo::save(deps.storage, lease_addr.clone(), &loan)
            .and_then(|()| {
                lpp.register_repay_loan(now, &loan, &payment)
                    .ok_or_else(|| {
                        ContractError::overflow_register_repayment(
                            "Registering repay loan when trying to repay loan",
                            now,
                            loan,
                            &payment,
                        )
                    })
            })
            .and_then(|()| lpp.save(deps.storage))
            .map(|()| payment)
    })?;

    let excess_received = payment.excess;
    let batch = if excess_received.is_zero() {
        Batch::default()
    } else {
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
    let config = Config::load(deps.storage)?;
    let bank = bank::account_view(&env.contract.address, deps.querier);
    let lpp = LiquidityPool::load(deps.storage, &config, &bank)?;

    match lpp.query_quote(quote, &env.block.time)? {
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
