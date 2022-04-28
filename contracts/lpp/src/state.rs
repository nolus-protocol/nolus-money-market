use std::cmp;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    coin, Addr, Coin, ContractInfoResponse, Decimal, Deps, DepsMut, Env, QueryRequest, StdResult,
    Storage, Timestamp, Uint128, Uint64, WasmQuery,
};

use crate::config::Config;
use crate::error::ContractError;
use crate::loan::Loan;
use crate::msg::{LoanResponse, QueryLoanResponse};
use cw_storage_plus::{Item, Map};

pub const NANOSECS_IN_YEAR: Uint128 = Uint128::new(365 * 24 * 60 * 60 * 1000 * 1000 * 1000);
pub const LPP: LiquidityPool = LiquidityPool::new("state", "config", "loans");

// TODO: evaluate fixed or rust_decimal instead of cosmwasm_std::Decimal
// https://docs.rs/fixed/latest/fixed/index.html
// https://docs.rs/rust_decimal/latest/rust_decimal/index.html
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub total_principal_due: Uint128,
    pub total_last_interest: Uint128,
    pub annual_interest_rate: Decimal,
    pub last_update_time: Timestamp,
}

pub struct LiquidityPool<'a> {
    state: Item<'a, State>,
    config: Item<'a, Config>,
    loans: Map<'a, Addr, Loan>,
}

impl<'a> LiquidityPool<'a> {
    pub const fn new(state_ns: &'a str, config_ns: &'a str, loans_ns: &'a str) -> Self {
        Self {
            state: Item::new(state_ns),
            config: Item::new(config_ns),
            loans: Map::new(loans_ns),
        }
    }

    pub fn init(&self, deps: DepsMut, denom: String, lease_code_id: Uint64) -> StdResult<()> {
        let config = Config::new(denom, lease_code_id);
        self.config.save(deps.storage, &config)?;
        Ok(())
    }

    pub fn balance(&self, deps: &Deps, env: &Env) -> StdResult<Coin> {
        let querier = deps.querier;
        let config = self.config.load(deps.storage)?;
        querier.query_balance(&env.contract.address, &config.denom)
    }

    pub fn validate_lease_addr(
        &self,
        deps: &Deps,
        lease_addr: Addr,
    ) -> Result<Addr, ContractError> {
        let querier = deps.querier;
        let config = self.config.load(deps.storage)?;
        let q_msg = QueryRequest::Wasm(WasmQuery::ContractInfo {
            contract_addr: lease_addr.to_string(),
        });
        let q_resp: ContractInfoResponse = querier.query(&q_msg)?;

        if q_resp.code_id != config.lease_code_id.u64() {
            Err(ContractError::ContractId {})
        } else {
            Ok(lease_addr)
        }
    }

    pub fn validate_denom(&self, deps: &Deps, coins: &Coin) -> Result<Uint128, ContractError> {
        let config = self.config.load(deps.storage)?;

        if config.denom != coins.denom {
            return Err(ContractError::Denom {
                contract_denom: config.denom,
                denom: coins.denom.clone(),
            });
        }
        Ok(coins.amount)
    }

    pub fn query_quote(
        &self,
        deps: &Deps,
        env: &Env,
        quote: Coin,
    ) -> Result<Option<Decimal>, ContractError> {
        let quote = self.validate_denom(deps, &quote)?;

        let balance = self.balance(deps, env)?;

        if quote > balance.amount {
            return Ok(None);
        }

        let State {
            total_principal_due,
            total_last_interest,
            annual_interest_rate,
            last_update_time,
        } = self.state.load(deps.storage)?;

        let Config {
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
            ..
        } = self.config.load(deps.storage)?;

        let total_interest = total_last_interest
            + total_principal_due * annual_interest_rate * dt(env, last_update_time)
                / NANOSECS_IN_YEAR;
        let total_liability = total_principal_due + quote + total_interest;

        let utilization = Decimal::from_ratio(total_liability, total_liability + balance.amount);

        let quote_interest_rate =
            base_interest_rate + (utilization - utilization_optimal) * addon_optimal_interest_rate;

        Ok(Some(quote_interest_rate))
    }

    pub fn query_loan(
        &self,
        storage: &dyn Storage,
        lease_addr: Addr,
    ) -> Result<QueryLoanResponse, ContractError> {
        let config = self.config.load(storage)?;

        let res = self.loans.may_load(storage, lease_addr)?.map(|loan| {
            let Loan {
                principal_due,
                annual_interest_rate,
                interest_paid,
            } = loan;

            LoanResponse {
                principal_due: coin(principal_due.u128(), config.denom.clone()),
                annual_interest_rate,
                interest_paid,
            }
        });

        Ok(res)
    }

    pub fn query_loan_outstanding_interest(
        &self,
        storage: &dyn Storage,
        lease_addr: Addr,
        outstanding_time: Timestamp,
    ) -> Result<Option<Coin>, ContractError> {
        let maybe_loan = self.loans.may_load(storage, lease_addr)?;
        let config = self.config.load(storage)?;

        if let Some(loan) = maybe_loan {
            let delta_t: Uint128 = (cmp::max(outstanding_time.nanos(), loan.interest_paid.nanos())
                - loan.interest_paid.nanos())
            .into();
            let outstanding_interest_amount =
                loan.principal_due * loan.annual_interest_rate * delta_t / NANOSECS_IN_YEAR;
            Ok(Some(coin(outstanding_interest_amount.u128(), config.denom)))
        } else {
            Ok(None)
        }
    }

    pub fn try_open_loan(
        &self,
        deps: DepsMut,
        env: Env,
        lease_addr: Addr,
        amount: Coin,
    ) -> Result<(), ContractError> {
        let checked_lease_addr = self.validate_lease_addr(&deps.as_ref(), lease_addr)?;
        let current_time = env.block.time;

        let annual_interest_rate = match self.query_quote(&deps.as_ref(), &env, amount.clone())? {
            Some(rate) => Ok(rate),
            None => Err(ContractError::NoLiquidity {}),
        }?;

        if self.loans.has(deps.storage, checked_lease_addr.clone()) {
            return Err(ContractError::LoanExists {});
        }

        let loan = Loan {
            principal_due: amount.amount,
            annual_interest_rate,
            interest_paid: current_time,
        };

        self.loans
            .save(deps.storage, checked_lease_addr, &loan)?;

        self.state
            .update(deps.storage, |mut state| -> Result<State, ContractError> {
                let dt = dt(&env, state.last_update_time);

                state.total_last_interest +=
                    state.total_principal_due * state.annual_interest_rate * dt / NANOSECS_IN_YEAR;

                state.annual_interest_rate = Decimal::from_ratio(
                    state.annual_interest_rate * state.total_principal_due
                        + loan.annual_interest_rate * loan.principal_due,
                    state.total_principal_due + loan.principal_due,
                );

                state.total_principal_due += amount.amount;

                state.last_update_time = current_time;

                Ok(state)
            })?;

        Ok(())
    }

    /// return amount of lpp currency to pay back to lease_addr
    pub fn try_repay_loan(
        &self,
        deps: DepsMut,
        env: Env,
        lease_addr: Addr,
        funds: Vec<Coin>,
    ) -> Result<Coin, ContractError> {
        if funds.len() != 1 {
            return Err(ContractError::FundsLen {});
        }

        let checked_lease_addr = self.validate_lease_addr(&deps.as_ref(), lease_addr)?;
        let repay_amount = self.validate_denom(&deps.as_ref(), &funds[0])?;

        let loan = self.loans.load(deps.storage, checked_lease_addr.clone())?;

        let time_delta = dt(&env, loan.interest_paid);
        let loan_interest_due =
            loan.principal_due * loan.annual_interest_rate * time_delta / NANOSECS_IN_YEAR;
        let loan_interest_payment = cmp::min(loan_interest_due, repay_amount);
        let loan_principal_payment =
            cmp::max(repay_amount - loan_interest_payment, loan.principal_due);
        let excess_received = repay_amount - loan_interest_payment - loan_principal_payment;

        if loan.principal_due == loan_principal_payment {
            self.loans.remove(deps.storage, checked_lease_addr);
        } else {
            self.loans.update(
                deps.storage,
                checked_lease_addr,
                |loan| -> Result<Loan, ContractError> {
                    let mut loan = loan.ok_or(ContractError::NoLoan {})?;
                    loan.principal_due -= loan_principal_payment;

                    let interest_paid_delta: u64 = (loan_interest_payment / loan_interest_due
                        * time_delta)
                        .u128()
                        .try_into()
                        .expect("math overflow");
                    loan.interest_paid =
                        Timestamp::from_nanos(loan.interest_paid.nanos() + interest_paid_delta);

                    Ok(loan)
                },
            )?;
        }

        self.state
            .update(deps.storage, |mut state| -> Result<State, ContractError> {
                // TODO: refactoring
                state.total_last_interest += state.total_principal_due
                    * state.annual_interest_rate
                    * dt(&env, state.last_update_time)
                    / NANOSECS_IN_YEAR;

                state.annual_interest_rate = Decimal::from_ratio(
                    state.annual_interest_rate * state.total_principal_due
                        - loan.annual_interest_rate * loan_principal_payment,
                    state.total_principal_due - loan_principal_payment,
                );

                state.total_principal_due -= loan_principal_payment;

                state.last_update_time = env.block.time;

                Ok(state)
            })?;

        let denom = self.config.load(deps.storage)?.denom;

        Ok(coin(excess_received.u128(), denom))
    }
}

/// Time difference in nanosecs between current time from `env.block.time` and timestamp.
fn dt(env: &Env, time: Timestamp) -> Uint128 {
    let ct = env.block.time.nanos();
    let t = time.nanos();
    assert!(ct > t);
    Uint128::new((ct - t).into())
}
