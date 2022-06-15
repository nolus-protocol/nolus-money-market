#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, Storage,
    Timestamp, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::lpp::LiquidityPool;
use crate::msg::{
    BalanceResponse, ExecuteMsg, InstantiateMsg, LppBalanceResponse, PriceResponse,
    QueryLoanOutstandingInterestResponse, QueryLoanResponse, QueryMsg, QueryQuoteResponse,
    RewardsResponse,
};
use crate::state::Deposit;

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// TODO: move to some global config package
pub const NOLUS_DENOM: &str = "uNLS";

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    LiquidityPool::store(deps.storage, msg.denom, msg.lease_code_id)?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let sender = info.sender;
    let funds = info.funds;

    match msg {
        ExecuteMsg::OpenLoan { amount } => try_open_loan(deps, env, sender, amount),
        ExecuteMsg::RepayLoan => try_repay_loan(deps, env, sender, funds),
        ExecuteMsg::Deposit => try_deposit(deps, env, sender, funds),
        ExecuteMsg::Burn { amount } => try_withdraw(deps, env, sender, amount),
        ExecuteMsg::DistributeRewards => try_distribute_rewards(deps, funds),
        ExecuteMsg::ClaimRewards { other_recipient } => {
            try_claim_rewards(deps, sender, other_recipient)
        }
    }
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        QueryMsg::Quote { amount } => to_binary(&query_quote(&deps, &env, amount)?),
        QueryMsg::Loan { lease_addr } => to_binary(&query_loan(deps.storage, env, lease_addr)?),
        QueryMsg::LoanOutstandingInterest {
            lease_addr,
            outstanding_time,
        } => to_binary(&query_loan_outstanding_interest(
            deps.storage,
            lease_addr,
            outstanding_time,
        )?),
        QueryMsg::Price => to_binary(&query_ntoken_price(deps, env)?),
        QueryMsg::Balance { address } => to_binary(&query_balance(deps.storage, address)?),
        QueryMsg::LppBalance => to_binary(&query_lpp_balance(deps, env)?),
        QueryMsg::Rewards { address } => to_binary(&query_rewards(deps.storage, address)?),
    }?;

    Ok(res)
}

fn try_open_loan(
    deps: DepsMut,
    env: Env,
    lease_addr: Addr,
    amount: Coin,
) -> Result<Response, ContractError> {
    let mut lpp = LiquidityPool::load(deps.storage)?;
    lpp.validate_lease_addr(&deps.as_ref(), &lease_addr)?;
    lpp.try_open_loan(deps, env, lease_addr.clone(), amount.clone())?;

    let transfer_msg = BankMsg::Send {
        to_address: lease_addr.to_string(),
        amount: vec![amount],
    };

    let response = Response::new()
        .add_attribute("method", "try_open_loan")
        .add_message(transfer_msg);

    Ok(response)
}

fn try_repay_loan(
    deps: DepsMut,
    env: Env,
    lease_addr: Addr,
    funds: Vec<Coin>,
) -> Result<Response, ContractError> {
    let mut lpp = LiquidityPool::load(deps.storage)?;
    lpp.validate_lease_addr(&deps.as_ref(), &lease_addr)?;
    let excess_received = lpp.try_repay_loan(deps, env, lease_addr.clone(), funds)?;

    let mut response = Response::new().add_attribute("method", "try_repay_loan");

    if !excess_received.is_zero() {
        let payment = lpp.pay(lease_addr, excess_received);
        response = response.add_message(payment);
    }

    Ok(response)
}

fn try_deposit(
    deps: DepsMut,
    env: Env,
    lender_addr: Addr,
    funds: Vec<Coin>,
) -> Result<Response, ContractError> {
    if funds.len() != 1 {
        return Err(ContractError::FundsLen {});
    }

    let lpp = LiquidityPool::load(deps.storage)?;
    let amount = lpp.try_into_amount(funds[0].clone())?;
    let price = lpp.calculate_price(&deps.as_ref(), &env)?;
    Deposit::load(deps.storage, lender_addr)?.deposit(deps.storage, amount, price)?;

    Ok(Response::new().add_attribute("method", "try_deposit"))
}

fn try_withdraw(
    deps: DepsMut,
    env: Env,
    lender_addr: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let lpp = LiquidityPool::load(deps.storage)?;
    let price = lpp.calculate_price(&deps.as_ref(), &env)?.get();
    let payment = amount * price;

    if lpp.balance(&deps.as_ref(), &env)?.amount < payment {
        return Err(ContractError::NoLiquidity {});
    }

    let maybe_reward =
        Deposit::load(deps.storage, lender_addr.clone())?.withdraw(deps.storage, amount)?;

    let payment_msg = lpp.pay(lender_addr, payment);

    let mut response = Response::new()
        .add_attribute("method", "try_withdraw")
        .add_message(payment_msg);

    // TODO: refactor pay to avoid sending 2 msgs
    if let Some(reward_msg) = maybe_reward {
        response = response.add_message(reward_msg)
    }

    Ok(response)
}

fn try_distribute_rewards(
    deps: DepsMut,
    funds: Vec<Coin>,
) -> Result<Response, ContractError> {
    match funds.iter().find(|&coin| coin.denom == NOLUS_DENOM) {
        Some(coin) => Deposit::distribute_rewards(deps, coin.to_owned())?,
        None => {
            return Err(ContractError::CustomError {
                val: "Rewards are supported only in native currency".to_string(),
            })
        }
    }

    Ok(Response::new().add_attribute("method", "try_distribute_rewards"))
}

fn try_claim_rewards(
    deps: DepsMut,
    addr: Addr,
    other_recipient: Option<Addr>,
) -> Result<Response, ContractError> {
    let mut deposit = Deposit::load(deps.storage, addr)?;
    let reward_msg = deposit.claim_rewards(deps.storage, other_recipient)?;

    let response = Response::new()
        .add_attribute("method", "try_claim_rewards")
        .add_message(reward_msg);

    Ok(response)
}

fn query_quote(deps: &Deps, env: &Env, quote: Coin) -> Result<QueryQuoteResponse, ContractError> {
    let lpp = LiquidityPool::load(deps.storage)?;
    match lpp.query_quote(deps, env, quote)? {
        Some(quote) => Ok(QueryQuoteResponse::QuoteInterestRate(quote)),
        None => Ok(QueryQuoteResponse::NoLiquidity),
    }
}

fn query_loan(
    storage: &dyn Storage,
    env: Env,
    lease_addr: Addr,
) -> Result<QueryLoanResponse, ContractError> {
    LiquidityPool::load(storage)?.query_loan(storage, &env, lease_addr)
}

fn query_loan_outstanding_interest(
    storage: &dyn Storage,
    loan: Addr,
    outstanding_time: Timestamp,
) -> Result<QueryLoanOutstandingInterestResponse, ContractError> {
    let interest = LiquidityPool::load(storage)?.query_loan_outstanding_interest(
        storage,
        loan,
        outstanding_time,
    )?;

    Ok(interest)
}

fn query_lpp_balance(deps: Deps, env: Env) -> Result<LppBalanceResponse, ContractError> {
    let lpp = LiquidityPool::load(deps.storage)?;
    Ok(lpp.query_lpp_balance(&deps, &env)?)
}

fn query_ntoken_price(deps: Deps, env: Env) -> Result<PriceResponse, ContractError> {
    let lpp = LiquidityPool::load(deps.storage)?;
    let price = lpp.calculate_price(&deps, &env)?.into();

    Ok(price)
}

fn query_balance(storage: &dyn Storage, addr: Addr) -> Result<BalanceResponse, ContractError> {
    let balance = Deposit::query_balance_nlpn(storage, addr)?.unwrap_or_default();
    Ok(BalanceResponse { balance })
}

fn query_rewards(storage: &dyn Storage, addr: Addr) -> Result<RewardsResponse, ContractError> {
    let deposit = Deposit::load(storage, addr)?;
    let rewards = deposit.query_rewards(storage)?;
    Ok(RewardsResponse { rewards })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, Uint64};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "ust"));

        let msg = InstantiateMsg {
            denom: "ust".into(),
            lease_code_id: Uint64::new(1000),
        };
        let info = mock_info("creator", &coins(1000, "ust"));

        instantiate(deps.as_mut(), mock_env(), info, msg).expect("can't instantiate");
    }
}
