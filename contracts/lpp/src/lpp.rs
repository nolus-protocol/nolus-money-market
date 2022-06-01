use cosmwasm_std::{
    coin, Addr, Coin, ContractInfoResponse, Decimal, Deps, DepsMut, Env, QueryRequest, StdResult,
    Storage, Uint128, WasmQuery, BankMsg,
};

use crate::error::ContractError;
use crate::msg::LppBalanceResponse;
use crate::state::{Config, Loan, Total, Deposit};
use crate::calc::{dt, interest};

pub struct NTokenPrice(Decimal);
impl NTokenPrice {
    pub fn get(&self) -> Decimal {
        self.0
    }
}

pub struct LiquidityPool {
    config: Config,
    total: Total,
}

impl LiquidityPool {

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        let config = Config::load(storage)?;
        let total = Total::load(storage)?;

        Ok(LiquidityPool {config, total})
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn balance(&self, deps: &Deps, env: &Env) -> StdResult<Coin> {
        let querier = deps.querier;
        querier.query_balance(&env.contract.address, &self.config.denom)
    }

    pub fn total_lpn(&self, deps: &Deps, env: &Env) -> StdResult<Uint128> {

        let res = self.balance(deps, env)?.amount
            + self.total.total_principal_due()
            + self.total.total_interest_due_by_now(env);

        Ok(res)
    }

    pub fn query_lpp_balance(&self, deps: &Deps,env: &Env) -> StdResult<LppBalanceResponse> {
        let balance = self.balance(deps, env)?;
        let denom = &self.config.denom;

        let total_principal_due_amount = self.total.total_principal_due();
        let total_principal_due = coin(total_principal_due_amount.u128(), denom);

        let total_interest_due_amount = self.total.total_interest_due_by_now(&env);
        let total_interest_due = coin(total_interest_due_amount.u128(), denom);

        Ok(LppBalanceResponse{balance, total_principal_due, total_interest_due })
    }

    pub fn calculate_price(&self, deps: &Deps, env: &Env) -> StdResult<NTokenPrice> {

        let balance_nlpn = Deposit::balance_nlpn(deps.storage)?;

        let price = if balance_nlpn.is_zero() {
            self.config.initial_derivative_price
        } else {
            Decimal::from_ratio(
                self.total_lpn(deps, env)?,
                balance_nlpn)
        };

        Ok(NTokenPrice(price))
    }

    pub fn validate_lease_addr(
        &self,
        deps: &Deps,
        lease_addr: &Addr,
    ) -> Result<(), ContractError> {
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
    ) -> Result<Option<Decimal>, ContractError> {
        let quote = self.try_into_amount(quote)?;

        let balance = self.balance(deps, env)?
            .amount;

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

        let total_interest = total_interest_due
            + interest(total_principal_due, annual_interest_rate, dt(env, last_update_time));

        let total_liability_past_quote = total_principal_due + quote + total_interest;

        let total_balance_past_quote = balance - quote;

        let utilization = Decimal::from_ratio(total_liability_past_quote, total_liability_past_quote + total_balance_past_quote);

        let quote_interest_rate =
            base_interest_rate + utilization * addon_optimal_interest_rate - utilization_optimal * addon_optimal_interest_rate;

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

        Loan::open(deps.storage, lease_addr, amount.amount, annual_interest_rate, current_time)?;

        self.total
            .borrow(&env, amount.amount, annual_interest_rate)
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
        let (loan_principal_payment, excess_received) = loan.repay(deps.storage, &env, repay_amount)?;

        self.total.repay(&env, loan_principal_payment, loan_annual_interest_rate)
            .store(deps.storage)?;

        Ok(excess_received)
    }
}



#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::testing::{self, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{Uint64, Timestamp, coin};
    use crate::calc::NANOSECS_IN_YEAR;
    use crate::state::LoanData;

    #[test]
    fn test_dt() {
        let mut env = testing::mock_env();
        env.block.time = Timestamp::from_nanos(20);
        assert_eq!(dt(&env, Timestamp::from_nanos(10)), 10u128.into());
    }

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

        let lpp = LiquidityPool::load(deps.as_mut().storage)
            .expect("can't load LiquidityPool");

        let balance = lpp.balance(&deps.as_ref(), &env)
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

        let lpp = LiquidityPool::load(deps.as_mut().storage)
            .expect("can't load LiquidityPool");

        // same denom
        let checked = lpp.try_into_amount(coin(amount, "uust"))
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

        let mut lpp = LiquidityPool::load(deps.as_mut().storage)
            .expect("can't load LiquidityPool");

        env.block.time = Timestamp::from_nanos(10);

        let result = lpp.query_quote(&deps.as_ref(), &env, coin(5_000_000, "uust"))
            .expect("can't query quote")
            .expect("should return some interest_rate");

        let interest_rate = Decimal::percent(7) + Decimal::percent(50)*Decimal::percent(2) - Decimal::percent(70)*Decimal::percent(2);

        assert_eq!(result, interest_rate);

        lpp.try_open_loan(deps.as_mut(), env.clone(), loan, coin(5_000_000, "uust"))
            .expect("can't open loan");
        deps.querier.update_balance(MOCK_CONTRACT_ADDR, vec![coin(5_000_000, "uust")]);

        // wait for year/10
        env.block.time = Timestamp::from_nanos((10 + NANOSECS_IN_YEAR.u128()/10).try_into().unwrap());

        let interest_rate = Decimal::percent(7) + Decimal::from_ratio(6033u128,10033u128)*Decimal::percent(2) - Decimal::percent(70)*Decimal::percent(2);

        let result = lpp.query_quote(&deps.as_ref(), &env, coin(1_000_000, "uust"))
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

        let annual_interest_rate = Decimal::from_ratio(66u128,1000u128);

        Config::new("uust".into(), lease_code_id)
            .store(deps.as_mut().storage)
            .expect("can't initialize Config");
        Total::default()
            .store(deps.as_mut().storage)
            .expect("can't initialize Total");

        let mut lpp = LiquidityPool::load(deps.as_mut().storage)
            .expect("can't load LiquidityPool");

        // doesn't exist
        let loan_response = Loan::query(deps.as_ref().storage, loan.clone())
            .expect("can't query loan");
        assert_eq!(loan_response, None);


        env.block.time = Timestamp::from_nanos(10);

        lpp.try_open_loan(deps.as_mut(), env.clone(), loan.clone(), coin(5_000_000, "uust"))
            .expect("can't open loan");
        deps.querier.update_balance(MOCK_CONTRACT_ADDR, vec![coin(5_000_000, "uust")]);

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
        env.block.time = Timestamp::from_nanos((10 + NANOSECS_IN_YEAR.u128()/10).try_into().unwrap());

        // pay interest for year/10
        let payment = Loan::query_outstanding_interest(deps.as_ref().storage, loan.clone(), env.block.time)
            .expect("can't query outstanding interest")
            .expect("should be some coins");


        let repay = lpp.try_repay_loan(deps.as_mut(), env.clone(), loan.clone(), vec![coin(payment.u128(), "uust")])
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

        // wait for another year/10
        env.block.time = Timestamp::from_nanos((10 + 2*NANOSECS_IN_YEAR.u128()/10).try_into().unwrap());

        // pay everything + excess
        let payment = Loan::query_outstanding_interest(deps.as_ref().storage, loan.clone(), env.block.time)
            .expect("can't query outstanding interest")
            .expect("should be some coins").u128()
                + 5_000_000
                + 100;

        let repay = lpp.try_repay_loan(deps.as_mut(), env, loan, vec![coin(payment, "uust")])
            .expect("can't repay loan");

        assert_eq!(repay, 100u128.into());

    }

}
