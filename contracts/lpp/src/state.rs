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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct State {
    pub total_principal_due: Uint128,
    pub total_interest_due: Uint128,
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

        let state = State::default();
        self.state.save(deps.storage, &state)?;

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
        lease_addr: &Addr,
    ) -> Result<(), ContractError> {
        let querier = deps.querier;
        let config = self.config.load(deps.storage)?;
        let q_msg = QueryRequest::Wasm(WasmQuery::ContractInfo {
            contract_addr: lease_addr.to_string(),
        });
        let q_resp: ContractInfoResponse = querier.query(&q_msg)?;

        if q_resp.code_id != config.lease_code_id.u64() {
            Err(ContractError::ContractId {})
        } else {
            Ok(())
        }
    }

    // TODO: introduce a Denom type that would list all Nolus supported currencies.
    /// checks `coins` denom vs config, converts Coin into it's amount;
    pub fn try_into_amount(&self, deps: &Deps, coins: Coin) -> Result<Uint128, ContractError> {
        let config = self.config.load(deps.storage)?;

        if config.denom != coins.denom {
            return Err(ContractError::Denom {
                contract_denom: config.denom,
                denom: coins.denom,
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
        let quote = self.try_into_amount(deps, quote)?;

        let balance = self.balance(deps, env)?
            .amount;

        if quote > balance {
            return Ok(None);
        }

        let State {
            total_principal_due,
            total_interest_due,
            annual_interest_rate,
            last_update_time,
        } = self.state.load(deps.storage)?;

        let Config {
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
            ..
        } = self.config.load(deps.storage)?;

        let total_interest = total_interest_due
            + interest(total_principal_due, annual_interest_rate, dt(env, last_update_time));

        let total_liability_past_quote = total_principal_due + quote + total_interest;

        let total_balance_past_quote = balance - quote;

        let utilization = Decimal::from_ratio(total_liability_past_quote, total_liability_past_quote + total_balance_past_quote);

        let quote_interest_rate =
            base_interest_rate + utilization * addon_optimal_interest_rate - utilization_optimal * addon_optimal_interest_rate;

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

            let outstanding_interest_amount = interest(loan.principal_due, loan.annual_interest_rate, delta_t);

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
        let current_time = env.block.time;

        let annual_interest_rate = match self.query_quote(&deps.as_ref(), &env, amount.clone())? {
            Some(rate) => Ok(rate),
            None => Err(ContractError::NoLiquidity {}),
        }?;

        if self.loans.has(deps.storage, lease_addr.clone()) {
            return Err(ContractError::LoanExists {});
        }

        let loan = Loan {
            principal_due: amount.amount,
            annual_interest_rate,
            interest_paid: current_time,
        };

        self.loans
            .save(deps.storage, lease_addr, &loan)?;

        self.state
            .update(deps.storage, |mut state| -> Result<State, ContractError> {
                let dt = dt(&env, state.last_update_time);

                state.total_interest_due += interest(state.total_principal_due, state.annual_interest_rate, dt);

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

        let repay_amount = self.try_into_amount(&deps.as_ref(), funds[0].clone())?;

        let loan = self.loans.load(deps.storage, lease_addr.clone())?;

        let time_delta = dt(&env, loan.interest_paid);
        let loan_interest_due = interest(loan.principal_due, loan.annual_interest_rate, time_delta);
        let loan_interest_payment = cmp::min(loan_interest_due, repay_amount);
        let loan_principal_payment =
            cmp::min(repay_amount - loan_interest_payment, loan.principal_due);
        let excess_received = repay_amount - loan_interest_payment - loan_principal_payment;

        if loan.principal_due == loan_principal_payment {
            self.loans.remove(deps.storage, lease_addr);
        } else {
            self.loans.update(
                deps.storage,
                lease_addr,
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
                state.total_interest_due += interest(
                    state.total_principal_due,
                    state.annual_interest_rate,
                    dt(&env, state.last_update_time));

                state.annual_interest_rate = if state.total_principal_due == loan_principal_payment {
                    Decimal::zero()
                } else {
                    Decimal::from_ratio(
                        state.annual_interest_rate * state.total_principal_due
                            - loan.annual_interest_rate * loan_principal_payment,
                        state.total_principal_due - loan_principal_payment,
                    )
                };

                state.total_principal_due -= loan_principal_payment;

                state.last_update_time = env.block.time;

                Ok(state)
            })?;

        let denom = self.config.load(deps.storage)?.denom;

        Ok(coin(excess_received.u128(), denom))
    }
}

/// Time difference in nanosecs between current block time and timestamp.
fn dt(env: &Env, time: Timestamp) -> Uint128 {
    let ct = env.block.time.nanos();
    let t = time.nanos();
    assert!(ct > t);
    Uint128::new((ct - t).into())
}

/// Calculate interest
fn interest(due: Uint128, rate: Decimal, dt_nanos: Uint128) -> Uint128 {
    due*rate*dt_nanos/NANOSECS_IN_YEAR
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::testing::{self, MOCK_CONTRACT_ADDR};

    #[test]
    fn test_dt() {
        let mut env = testing::mock_env();
        env.block.time = Timestamp::from_nanos(20);
        assert_eq!(dt(&env, Timestamp::from_nanos(10)), 10u128.into());
    }

    #[test]
    fn test_balance() {
        let balance_mock = [coin(10_000_000, "uusdt")];
        let mut deps = testing::mock_dependencies_with_balance(&balance_mock);
        let env = testing::mock_env();
        let lease_code_id = Uint64::new(123);

        LPP.init(deps.as_mut(), "uusdt".into(), lease_code_id)
            .expect("can't initialize LiquidityPool");

        let balance = LPP.balance(&deps.as_ref(), &env)
            .expect("can't get balance");

        assert_eq!(balance, balance_mock[0]);
    }

    #[test]
    fn test_try_into_amount() {
        let mut deps = testing::mock_dependencies();
        let lease_code_id = Uint64::new(123);
        let amount = 10u128;

        LPP.init(deps.as_mut(), "uusdt".into(), lease_code_id)
            .expect("can't initialize LiquidityPool");

        // same denom
        let checked = LPP.try_into_amount(&deps.as_ref(), coin(amount, "uusdt"))
            .expect("can't validate denom");
        assert_eq!(checked, Uint128::new(amount));

        // err denom
        LPP.try_into_amount(&deps.as_ref(), coin(amount, "eth"))
            .expect_err("should not pass validation");

    }

    #[test]
    fn test_query_quote() {
        let balance_mock = [coin(10_000_000, "uusdt")];
        let mut deps = testing::mock_dependencies_with_balance(&balance_mock);
        let mut env = testing::mock_env();
        let loan = Addr::unchecked("loan");
        env.block.time = Timestamp::from_nanos(0);

        let lease_code_id = Uint64::new(123);

        LPP.init(deps.as_mut(), "uusdt".into(), lease_code_id)
            .expect("can't initialize LiquidityPool");

        env.block.time = Timestamp::from_nanos(10);

        let result = LPP.query_quote(&deps.as_ref(), &env, coin(5_000_000, "uusdt"))
            .expect("can't query quote")
            .expect("should return some interest_rate");

        let interest_rate = Decimal::percent(7) + Decimal::percent(50)*Decimal::percent(2) - Decimal::percent(70)*Decimal::percent(2);

        assert_eq!(result, interest_rate);

        LPP.try_open_loan(deps.as_mut(), env.clone(), loan, coin(5_000_000, "uusdt"))
            .expect("can't open loan");
        deps.querier.update_balance(MOCK_CONTRACT_ADDR, vec![coin(5_000_000, "uusdt")]);

        // wait for year/10
        env.block.time = Timestamp::from_nanos((10 + NANOSECS_IN_YEAR.u128()/10).try_into().unwrap());

        let interest_rate = Decimal::percent(7) + Decimal::from_ratio(6033u128,10033u128)*Decimal::percent(2) - Decimal::percent(70)*Decimal::percent(2);

        let result = LPP.query_quote(&deps.as_ref(), &env, coin(1_000_000, "uusdt"))
            .expect("can't query quote")
            .expect("should return some interest_rate");

        assert_eq!(result, interest_rate);

    }

    #[test]
    fn test_open_and_repay_loan() {
        let balance_mock = [coin(10_000_000, "uusdt")];
        let mut deps = testing::mock_dependencies_with_balance(&balance_mock);
        let mut env = testing::mock_env();
        let loan = Addr::unchecked("loan");
        env.block.time = Timestamp::from_nanos(0);
        let lease_code_id = Uint64::new(123);

        let annual_interest_rate = Decimal::from_ratio(66u128,1000u128);

        LPP.init(deps.as_mut(), "uusdt".into(), lease_code_id)
            .expect("can't initialize LiquidityPool");

        // doesn't exist
        let loan_response = LPP.query_loan(deps.as_ref().storage, loan.clone())
            .expect("can't query loan");
        assert_eq!(loan_response, None);


        env.block.time = Timestamp::from_nanos(10);

        LPP.try_open_loan(deps.as_mut(), env.clone(), loan.clone(), coin(5_000_000, "uusdt"))
            .expect("can't open loan");
        deps.querier.update_balance(MOCK_CONTRACT_ADDR, vec![coin(5_000_000, "uusdt")]);

        let loan_response = LPP.query_loan(deps.as_ref().storage, loan.clone())
            .expect("can't query loan")
            .expect("should be some response");

        let test_response = LoanResponse {
            principal_due: coin(5_000_000, "uusdt"),
            annual_interest_rate,
            interest_paid: env.block.time,
        };

        assert_eq!(loan_response, test_response);

        // wait for year/10
        env.block.time = Timestamp::from_nanos((10 + NANOSECS_IN_YEAR.u128()/10).try_into().unwrap());

        // pay interest for year/10
        let payment = LPP.query_loan_outstanding_interest(deps.as_ref().storage, loan.clone(), env.block.time)
            .expect("can't query outstanding interest")
            .expect("should be some coins");

        let repay = LPP.try_repay_loan(deps.as_mut(), env.clone(), loan.clone(), vec![payment])
            .expect("can't repay loan");

        assert_eq!(repay, coin(0, "uusdt"));

        let loan_response = LPP.query_loan(deps.as_ref().storage, loan.clone())
            .expect("can't query loan")
            .expect("should be some response");

        let test_response = LoanResponse {
            principal_due: coin(5_000_000, "uusdt"),
            annual_interest_rate,
            interest_paid: env.block.time,
        };

        assert_eq!(loan_response, test_response);

        // wait for another year/10
        env.block.time = Timestamp::from_nanos((10 + 2*NANOSECS_IN_YEAR.u128()/10).try_into().unwrap());

        // pay everything + excess
        let payment = LPP.query_loan_outstanding_interest(deps.as_ref().storage, loan.clone(), env.block.time)
            .expect("can't query outstanding interest")
            .expect("should be some coins").amount.u128()
                + 5_000_000
                + 100;

        let repay = LPP.try_repay_loan(deps.as_mut(), env, loan, vec![coin(payment, "uusdt")])
            .expect("can't repay loan");

        assert_eq!(repay, coin(100, "uusdt"));

    }

}
