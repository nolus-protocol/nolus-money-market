use cosmwasm_std::{
    coin, Addr, BankMsg, Coin, ContractInfoResponse, Decimal, Deps, DepsMut, Env, QueryRequest,
    StdResult, Storage, Timestamp, Uint128, Uint64, WasmQuery,
};

use crate::error::ContractError;
use crate::msg::{LoanResponse, LppBalanceResponse, OutstandingInterest, PriceResponse};
use crate::state::{Config, Deposit, Loan, Total};
use finance::duration::Duration;
use finance::interest::InterestPeriod;
use finance::percent::Percent;

pub struct NTokenPrice<'a> {
    price: Decimal,
    denom: &'a String,
}

impl<'a> NTokenPrice<'a> {
    pub fn get(&self) -> Decimal {
        self.price
    }
}

impl<'a> From<NTokenPrice<'a>> for PriceResponse {
    fn from(nprice: NTokenPrice) -> Self {
        PriceResponse {
            price: nprice.price,
            denom: nprice.denom.to_owned(),
        }
    }
}

pub struct LiquidityPool {
    config: Config,
    total: Total,
}

impl LiquidityPool {
    pub fn store(storage: &mut dyn Storage, denom: String, lease_code_id: Uint64) -> StdResult<()> {
        Config::new(denom, lease_code_id).store(storage)?;

        Total::default().store(storage)?;

        Ok(())
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        let config = Config::load(storage)?;
        let total = Total::load(storage)?;

        Ok(LiquidityPool { config, total })
    }

    // TODO: query parameters
    /*
        pub fn config(&self) -> &Config {
            &self.config
        }
    */

    pub fn balance(&self, deps: &Deps, env: &Env) -> StdResult<Coin> {
        let querier = deps.querier;
        querier.query_balance(&env.contract.address, &self.config.denom)
    }

    pub fn total_lpn(&self, deps: &Deps, env: &Env) -> StdResult<Uint128> {
        let res = self.balance(deps, env)?.amount
            + self.total.total_principal_due()
            + self.total.total_interest_due_by_now(env.block.time);

        Ok(res)
    }

    pub fn query_lpp_balance(&self, deps: &Deps, env: &Env) -> StdResult<LppBalanceResponse> {
        let balance = self.balance(deps, env)?;
        let denom = &self.config.denom;

        let total_principal_due_amount = self.total.total_principal_due();
        let total_principal_due = coin(total_principal_due_amount.u128(), denom);

        let total_interest_due_amount = self.total.total_interest_due_by_now(env.block.time);
        let total_interest_due = coin(total_interest_due_amount.u128(), denom);

        Ok(LppBalanceResponse {
            balance,
            total_principal_due,
            total_interest_due,
        })
    }

    pub fn calculate_price(&self, deps: &Deps, env: &Env) -> StdResult<NTokenPrice> {
        let balance_nlpn = Deposit::balance_nlpn(deps.storage)?;

        let price = if balance_nlpn.is_zero() {
            self.config.initial_derivative_price
        } else {
            Decimal::from_ratio(self.total_lpn(deps, env)?, balance_nlpn)
        };

        Ok(NTokenPrice {
            price,
            denom: &self.config.denom,
        })
    }

    pub fn validate_lease_addr(&self, deps: &Deps, lease_addr: &Addr) -> Result<(), ContractError> {
        let querier = deps.querier;
        let q_msg = QueryRequest::Wasm(WasmQuery::ContractInfo {
            contract_addr: lease_addr.to_string(),
        });
        let q_resp: ContractInfoResponse = querier.query(&q_msg)?;

        if q_resp.code_id != self.config.lease_code_id.u64() {
            Err(ContractError::ContractId {})
        } else {
            Ok(())
        }
    }

    pub fn pay(&self, addr: Addr, amount: Uint128) -> BankMsg {
        BankMsg::Send {
            to_address: addr.to_string(),
            amount: vec![coin(amount.u128(), &self.config.denom)],
        }
    }

    // TODO: introduce a Denom type that would list all Nolus supported currencies.
    /// checks `coins` denom vs config, converts Coin into it's amount;
    pub fn try_into_amount(&self, coins: Coin) -> Result<Uint128, ContractError> {
        if self.config.denom != coins.denom {
            return Err(ContractError::Denom {
                contract_denom: self.config.denom.clone(),
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
    ) -> Result<Option<Percent>, ContractError> {
        let quote = self.try_into_amount(quote)?;

        let balance = self.balance(deps, env)?.amount;

        if quote > balance {
            return Ok(None);
        }

        let total_principal_due = self.total.total_principal_due();
        let total_interest_due = self.total.total_interest_due();
        let annual_interest_rate = self.total.annual_interest_rate();
        let last_update_time = self.total.last_update_time();

        let Config {
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
            ..
        } = self.config;

        let total_interest = InterestPeriod::with_interest(annual_interest_rate)
            .from(last_update_time)
            .spanning(Duration::between(last_update_time, env.block.time))
            .interest(total_principal_due)
            + total_interest_due;

        let total_liability_past_quote = total_principal_due + quote + total_interest;

        let total_balance_past_quote = balance - quote;

        let utilization = Percent::from_permille(
            (1000 * total_liability_past_quote.u128()
                / (total_liability_past_quote + total_balance_past_quote).u128())
            .try_into()?,
        );

        let quote_interest_rate = base_interest_rate + addon_optimal_interest_rate.of(utilization)
            - addon_optimal_interest_rate.of(utilization_optimal);

        Ok(Some(quote_interest_rate))
    }

    pub fn try_open_loan(
        &mut self,
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

        Loan::open(
            deps.storage,
            lease_addr,
            amount.amount,
            annual_interest_rate,
            current_time,
        )?;

        self.total
            .borrow(env.block.time, amount.amount, annual_interest_rate)?
            .store(deps.storage)?;

        Ok(())
    }

    /// return amount of lpp currency to pay back to lease_addr
    pub fn try_repay_loan(
        &mut self,
        deps: DepsMut,
        env: Env,
        lease_addr: Addr,
        funds: Vec<Coin>,
    ) -> Result<Uint128, ContractError> {
        if funds.len() != 1 {
            return Err(ContractError::FundsLen {});
        }

        let repay_amount = self.try_into_amount(funds[0].clone())?;

        let mut loan = Loan::load(deps.storage, lease_addr)?;
        let loan_annual_interest_rate = loan.data().annual_interest_rate;
        let (loan_principal_payment, excess_received) =
            loan.repay(deps.storage, &env, repay_amount)?;

        self.total
            .repay(
                env.block.time,
                loan_principal_payment,
                loan_annual_interest_rate,
            )?
            .store(deps.storage)?;

        Ok(excess_received)
    }

    pub fn query_loan_outstanding_interest(
        &self,
        storage: &dyn Storage,
        addr: Addr,
        time: Timestamp,
    ) -> StdResult<Option<OutstandingInterest>> {
        let interest = Loan::query_outstanding_interest(storage, addr, time)?
            .map(|amount| OutstandingInterest(coin(amount.u128(), &self.config.denom)));

        Ok(interest)
    }

    pub fn query_loan(
        &self,
        storage: &dyn Storage,
        env: &Env,
        addr: Addr,
    ) -> Result<Option<LoanResponse>, ContractError> {
        let maybe_loan = Loan::query(storage, addr.clone())?;
        let maybe_interest_due =
            self.query_loan_outstanding_interest(storage, addr, env.block.time)?;
        let denom = &self.config.denom;
        maybe_loan
            .zip(maybe_interest_due)
            .map(|(loan, interest_due)| {
                Ok(LoanResponse {
                    principal_due: coin(loan.principal_due.u128(), denom),
                    interest_due: interest_due.0,
                    annual_interest_rate: loan.annual_interest_rate,
                    interest_paid: loan.interest_paid,
                })
            })
            .transpose()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::state::LoanData;
    use cosmwasm_std::testing::{self, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coin, Timestamp, Uint64};

    #[test]
    fn test_balance() {
        let balance_mock = [coin(10_000_000, "uust")];
        let mut deps = testing::mock_dependencies_with_balance(&balance_mock);
        let env = testing::mock_env();
        let lease_code_id = Uint64::new(123);

        Config::new("uust".into(), lease_code_id)
            .store(deps.as_mut().storage)
            .expect("can't initialize Config");
        Total::default()
            .store(deps.as_mut().storage)
            .expect("can't initialize Total");

        let lpp = LiquidityPool::load(deps.as_mut().storage).expect("can't load LiquidityPool");

        let balance = lpp
            .balance(&deps.as_ref(), &env)
            .expect("can't get balance");

        assert_eq!(balance, balance_mock[0]);
    }

    #[test]
    fn test_try_into_amount() {
        let mut deps = testing::mock_dependencies();
        let lease_code_id = Uint64::new(123);
        let amount = 10u128;

        Config::new("uust".into(), lease_code_id)
            .store(deps.as_mut().storage)
            .expect("can't initialize Config");
        Total::default()
            .store(deps.as_mut().storage)
            .expect("can't initialize Total");

        let lpp = LiquidityPool::load(deps.as_mut().storage).expect("can't load LiquidityPool");

        // same denom
        let checked = lpp
            .try_into_amount(coin(amount, "uust"))
            .expect("can't validate denom");
        assert_eq!(checked, Uint128::new(amount));

        // err denom
        lpp.try_into_amount(coin(amount, "eth"))
            .expect_err("should not pass validation");
    }

    #[test]
    fn test_query_quote() {
        let balance_mock = [coin(10_000_000, "uust")];
        let mut deps = testing::mock_dependencies_with_balance(&balance_mock);
        let mut env = testing::mock_env();
        let loan = Addr::unchecked("loan");
        env.block.time = Timestamp::from_nanos(0);

        let lease_code_id = Uint64::new(123);

        Config::new("uust".into(), lease_code_id)
            .store(deps.as_mut().storage)
            .expect("can't initialize Config");
        Total::default()
            .store(deps.as_mut().storage)
            .expect("can't initialize Total");

        let mut lpp = LiquidityPool::load(deps.as_mut().storage).expect("can't load LiquidityPool");

        env.block.time = Timestamp::from_nanos(10);

        let result = lpp
            .query_quote(&deps.as_ref(), &env, coin(5_000_000, "uust"))
            .expect("can't query quote")
            .expect("should return some interest_rate");

        let interest_rate = Percent::from_percent(7)
            + Percent::from_percent(50).of(Percent::from_percent(2))
            - Percent::from_percent(70).of(Percent::from_percent(2));

        assert_eq!(result, interest_rate);

        lpp.try_open_loan(deps.as_mut(), env.clone(), loan, coin(5_000_000, "uust"))
            .expect("can't open loan");
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, vec![coin(5_000_000, "uust")]);

        // wait for year/10
        env.block.time = Timestamp::from_nanos(10 + Duration::YEAR.nanos() / 10);

        let interest_rate = Percent::from_percent(7)
            + Percent::from_percent(2).of(Percent::from_permille(6033000u32 / 10033u32))
            - Percent::from_percent(2).of(Percent::from_percent(70));

        let result = lpp
            .query_quote(&deps.as_ref(), &env, coin(1_000_000, "uust"))
            .expect("can't query quote")
            .expect("should return some interest_rate");

        assert_eq!(result, interest_rate);
    }

    #[test]
    fn test_open_and_repay_loan() {
        let balance_mock = [coin(10_000_000, "uust")];
        let mut deps = testing::mock_dependencies_with_balance(&balance_mock);
        let mut env = testing::mock_env();
        let loan = Addr::unchecked("loan");
        env.block.time = Timestamp::from_nanos(0);
        let lease_code_id = Uint64::new(123);

        let annual_interest_rate = Percent::from_permille(66000u32 / 1000u32);

        Config::new("uust".into(), lease_code_id)
            .store(deps.as_mut().storage)
            .expect("can't initialize Config");
        Total::default()
            .store(deps.as_mut().storage)
            .expect("can't initialize Total");

        let mut lpp = LiquidityPool::load(deps.as_mut().storage).expect("can't load LiquidityPool");

        // doesn't exist
        let loan_response =
            Loan::query(deps.as_ref().storage, loan.clone()).expect("can't query loan");
        assert_eq!(loan_response, None);

        env.block.time = Timestamp::from_nanos(10);

        lpp.try_open_loan(
            deps.as_mut(),
            env.clone(),
            loan.clone(),
            coin(5_000_000, "uust"),
        )
        .expect("can't open loan");
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, vec![coin(5_000_000, "uust")]);

        let loan_response = Loan::query(deps.as_ref().storage, loan.clone())
            .expect("can't query loan")
            .expect("should be some response");

        let test_response = LoanData {
            principal_due: 5_000_000u128.into(),
            annual_interest_rate,
            interest_paid: env.block.time,
        };

        assert_eq!(loan_response, test_response);

        // wait for year/10
        env.block.time = Timestamp::from_nanos(10 + Duration::YEAR.nanos() / 10);

        // pay interest for year/10
        let payment =
            Loan::query_outstanding_interest(deps.as_ref().storage, loan.clone(), env.block.time)
                .expect("can't query outstanding interest")
                .expect("should be some coins");

        let repay = lpp
            .try_repay_loan(
                deps.as_mut(),
                env.clone(),
                loan.clone(),
                vec![coin(payment.u128(), "uust")],
            )
            .expect("can't repay loan");

        assert_eq!(repay, 0u128.into());

        let loan_response = Loan::query(deps.as_ref().storage, loan.clone())
            .expect("can't query loan")
            .expect("should be some response");

        let test_response = LoanData {
            principal_due: 5_000_000u128.into(),
            annual_interest_rate,
            interest_paid: env.block.time,
        };

        assert_eq!(loan_response, test_response);

        // an immediate repay after repay should pass (loan_interest_due==0 bug)
        lpp.try_repay_loan(
            deps.as_mut(),
            env.clone(),
            loan.clone(),
            vec![coin(0u128, "uust")],
        )
        .expect("can't repay loan");

        // wait for another year/10
        env.block.time = Timestamp::from_nanos(10 + 2 * Duration::YEAR.nanos() / 10);

        // pay everything + excess
        let payment =
            Loan::query_outstanding_interest(deps.as_ref().storage, loan.clone(), env.block.time)
                .expect("can't query outstanding interest")
                .expect("should be some coins")
                .u128()
                + 5_000_000
                + 100;

        let repay = lpp
            .try_repay_loan(deps.as_mut(), env, loan, vec![coin(payment, "uust")])
            .expect("can't repay loan");

        assert_eq!(repay, 100u128.into());
    }
}
