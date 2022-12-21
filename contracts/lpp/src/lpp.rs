use serde::{de::DeserializeOwned, Serialize};

use finance::{
    coin::Coin,
    currency::Currency,
    fraction::Fraction,
    percent::Percent,
    price::{self, Price},
    ratio::Rational,
};
use platform::{bank::BankView, contract};
use sdk::cosmwasm_std::{Addr, Deps, DepsMut, Env, StdResult, Storage, Timestamp};

use crate::{
    error::{ContractError, ContractResult},
    msg::{LoanResponse, LppBalanceResponse, OutstandingInterest, PriceResponse},
    nlpn::NLpn,
    state::{Config, Deposit, Loan, LoanData, Total},
};

pub struct NTokenPrice<LPN>
where
    LPN: 'static + Currency + Serialize + DeserializeOwned,
{
    price: Price<NLpn, LPN>,
}

impl<LPN> NTokenPrice<LPN>
where
    LPN: Currency + Serialize + DeserializeOwned,
{
    pub fn get(&self) -> Price<NLpn, LPN> {
        self.price
    }

    #[cfg(test)]
    pub fn mock(nlpn: Coin<NLpn>, lpn: Coin<LPN>) -> Self {
        Self {
            price: price::total_of(nlpn).is(lpn),
        }
    }
}

impl<LPN> From<NTokenPrice<LPN>> for PriceResponse<LPN>
where
    LPN: Currency + Serialize + DeserializeOwned,
{
    fn from(nprice: NTokenPrice<LPN>) -> Self {
        PriceResponse(nprice.price)
    }
}

pub struct LiquidityPool<LPN>
where
    LPN: Currency,
{
    config: Config,
    total: Total<LPN>,
}

impl<LPN> LiquidityPool<LPN>
where
    LPN: 'static + Currency + Serialize + DeserializeOwned,
{
    pub fn store(storage: &mut dyn Storage, config: Config) -> ContractResult<()> {
        config.store(storage)?;
        Total::<LPN>::new().store(storage)?;

        Ok(())
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        let config = Config::load(storage)?;
        let total = Total::load(storage)?;

        Ok(LiquidityPool { config, total })
    }

    pub fn balance(&self, deps: &Deps, env: &Env) -> Result<Coin<LPN>, ContractError> {
        use platform::bank::BankAccountView;
        let balance = BankView::my_account(env, &deps.querier).balance()?;

        Ok(balance)
    }

    pub fn total_lpn(&self, deps: &Deps, env: &Env) -> Result<Coin<LPN>, ContractError> {
        let res = self.balance(deps, env)?
            + self.total.total_principal_due()
            + self.total.total_interest_due_by_now(env.block.time);

        Ok(res)
    }

    pub fn query_lpp_balance(
        &self,
        deps: &Deps,
        env: &Env,
    ) -> Result<LppBalanceResponse<LPN>, ContractError> {
        let balance = self.balance(deps, env)?;

        let total_principal_due = self.total.total_principal_due();

        let total_interest_due = self.total.total_interest_due_by_now(env.block.time);

        let balance_nlpn = Deposit::balance_nlpn(deps.storage)?;

        Ok(LppBalanceResponse {
            balance,
            total_principal_due,
            total_interest_due,
            balance_nlpn,
        })
    }

    pub fn calculate_price(
        &self,
        deps: &Deps,
        env: &Env,
        received: Coin<LPN>,
    ) -> Result<NTokenPrice<LPN>, ContractError> {
        let balance_nlpn = Deposit::balance_nlpn(deps.storage)?;

        let price = if balance_nlpn.is_zero() {
            Config::initial_derivative_price()
        } else {
            price::total_of(balance_nlpn).is(self.total_lpn(deps, env)? - received)
        };

        Ok(NTokenPrice { price })
    }

    pub fn validate_lease_addr(&self, deps: &Deps, lease_addr: &Addr) -> Result<(), ContractError> {
        contract::validate_code_id(&deps.querier, lease_addr, self.config.lease_code_id().u64())
            .map_err(ContractError::from)
    }

    pub fn withdraw_lpn(
        &self,
        deps: &Deps,
        env: &Env,
        amount_nlpn: Coin<NLpn>,
    ) -> Result<Coin<LPN>, ContractError> {
        let price = self.calculate_price(deps, env, Coin::new(0))?.get();
        let amount_lpn = price::total(amount_nlpn, price);

        if self.balance(deps, env)? < amount_lpn {
            return Err(ContractError::NoLiquidity {});
        }

        Ok(amount_lpn)
    }

    pub fn query_quote(
        &self,
        deps: &Deps,
        env: &Env,
        quote: Coin<LPN>,
    ) -> Result<Option<Percent>, ContractError> {
        let balance = self.balance(deps, env)?;

        if quote > balance {
            return Ok(None);
        }

        let interest_rate = self.config.borrow_rate();

        let total_principal_due = self.total.total_principal_due();
        let total_interest = self.total.total_interest_due_by_now(env.block.time);
        let total_liability_past_quote = total_principal_due + quote + total_interest;
        let total_balance_past_quote = balance - quote;

        // utilization % / utilization_optimal %
        let utilization_rel = Rational::new(
            total_liability_past_quote,
            interest_rate
                .utilization_optimal()
                .of(total_liability_past_quote + total_balance_past_quote),
        );

        let quote_interest_rate = interest_rate.base_interest_rate()
            + Fraction::<Coin<LPN>>::of(
                &utilization_rel,
                interest_rate.addon_optimal_interest_rate(),
            );

        Ok(Some(quote_interest_rate))
    }

    pub fn try_open_loan(
        &mut self,
        deps: &mut DepsMut,
        env: &Env,
        lease_addr: Addr,
        amount: Coin<LPN>,
    ) -> Result<Percent, ContractError> {
        if amount.is_zero() {
            return Err(ContractError::ZeroLoanAmount);
        }

        let current_time = env.block.time;

        let annual_interest_rate = match self.query_quote(&deps.as_ref(), env, amount)? {
            Some(rate) => Ok(rate),
            None => Err(ContractError::NoLiquidity {}),
        }?;

        Loan::open(
            deps.storage,
            lease_addr,
            amount,
            annual_interest_rate,
            current_time,
        )?;

        self.total
            .borrow(env.block.time, amount, annual_interest_rate)?
            .store(deps.storage)?;

        Ok(annual_interest_rate)
    }

    /// return amount of lpp currency to pay back to lease_addr
    pub fn try_repay_loan(
        &mut self,
        deps: &mut DepsMut,
        env: &Env,
        lease_addr: Addr,
        repay_amount: Coin<LPN>,
    ) -> Result<Coin<LPN>, ContractError> {
        let loan = Loan::load(deps.storage, lease_addr)?;
        let loan_annual_interest_rate = loan.data().annual_interest_rate;
        let payment = loan.repay(deps.storage, env.block.time, repay_amount)?;

        self.total
            .repay(
                env.block.time,
                payment.interest,
                payment.principal,
                loan_annual_interest_rate,
            )?
            .store(deps.storage)?;

        Ok(payment.excess)
    }

    pub fn query_loan_outstanding_interest(
        &self,
        storage: &dyn Storage,
        addr: Addr,
        time: Timestamp,
    ) -> StdResult<Option<OutstandingInterest<LPN>>> {
        let interest =
            Loan::query_outstanding_interest(storage, addr, time)?.map(OutstandingInterest);

        Ok(interest)
    }

    pub fn query_loan(
        &self,
        storage: &dyn Storage,
        env: &Env,
        addr: Addr,
    ) -> Result<Option<LoanResponse<LPN>>, ContractError> {
        let maybe_loan = Loan::query(storage, addr.clone())?;
        let maybe_interest_due =
            self.query_loan_outstanding_interest(storage, addr, env.block.time)?;
        maybe_loan
            .zip(maybe_interest_due)
            .map(|(loan, interest_due): (LoanData<LPN>, _)| {
                Ok(LoanResponse {
                    principal_due: loan.principal_due,
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
    use access_control::SingleUserAccess;
    use finance::{duration::Duration, percent::Units, price, test::currency::Usdc};
    use platform::coin_legacy;
    use sdk::cosmwasm_std::{
        testing::{self, MOCK_CONTRACT_ADDR},
        Addr, Coin as CwCoin, Timestamp, Uint64,
    };

    use crate::{
        borrow::InterestRate,
        state::{Config, Deposit, Total},
    };

    use super::*;

    type TheCurrency = Usdc;

    const BASE_INTEREST_RATE: Percent = Percent::from_permille(70);
    const UTILIZATION_OPTIMAL: Percent = Percent::from_permille(700);
    const ADDON_OPTIMAL_INTEREST_RATE: Percent = Percent::from_permille(20);

    #[test]
    fn test_balance() {
        let balance_mock = coin_cw(10_000_000);
        let mut deps = testing::mock_dependencies_with_balance(&[balance_mock.clone()]);
        let env = testing::mock_env();
        let lease_code_id = Uint64::new(123);
        let admin = Addr::unchecked("admin");

        SingleUserAccess::new_contract_owner(admin)
            .store(deps.as_mut().storage)
            .unwrap();

        Config::new(
            balance_mock.denom.clone(),
            lease_code_id,
            InterestRate::new(
                BASE_INTEREST_RATE,
                UTILIZATION_OPTIMAL,
                ADDON_OPTIMAL_INTEREST_RATE,
            )
            .expect("Couldn't construct interest rate value!"),
        )
        .store(deps.as_mut().storage)
        .expect("Failed to store Config!");
        Total::<TheCurrency>::new()
            .store(deps.as_mut().storage)
            .expect("can't initialize Total");

        let lpp = LiquidityPool::<TheCurrency>::load(deps.as_mut().storage)
            .expect("can't load LiquidityPool");

        let balance = lpp
            .balance(&deps.as_ref(), &env)
            .expect("can't get balance");

        assert_eq!(balance, balance_mock.amount.into());
    }

    #[test]
    fn test_query_quote() {
        let balance_mock = coin_cw(10_000_000);
        let mut deps = testing::mock_dependencies_with_balance(&[balance_mock.clone()]);
        let mut env = testing::mock_env();
        let admin = Addr::unchecked("admin");
        let loan = Addr::unchecked("loan");
        env.block.time = Timestamp::from_nanos(0);

        let lease_code_id = Uint64::new(123);

        SingleUserAccess::new_contract_owner(admin)
            .store(deps.as_mut().storage)
            .unwrap();

        Config::new(
            balance_mock.denom,
            lease_code_id,
            InterestRate::new(
                BASE_INTEREST_RATE,
                UTILIZATION_OPTIMAL,
                ADDON_OPTIMAL_INTEREST_RATE,
            )
            .expect("Couldn't construct interest rate value!"),
        )
        .store(deps.as_mut().storage)
        .expect("Failed to store Config!");
        Total::<TheCurrency>::new()
            .store(deps.as_mut().storage)
            .expect("can't initialize Total");

        let mut lpp = LiquidityPool::<TheCurrency>::load(deps.as_mut().storage)
            .expect("can't load LiquidityPool");

        env.block.time = Timestamp::from_nanos(10);

        let result = lpp
            .query_quote(&deps.as_ref(), &env, Coin::new(7_700_000))
            .expect("can't query quote")
            .expect("should return some interest_rate");

        assert_eq!(result, Percent::from_permille(92));

        lpp.try_open_loan(&mut deps.as_mut(), &env, loan, Coin::new(7_000_000))
            .expect("can't open loan");
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, vec![coin_cw(3_000_000)]);

        // wait for a year
        env.block.time = Timestamp::from_nanos(10 + Duration::YEAR.nanos());

        let result = lpp
            .query_quote(&deps.as_ref(), &env, Coin::new(1_000_000))
            .expect("can't query quote")
            .expect("should return some interest_rate");

        assert_eq!(result, Percent::from_permille(93));
    }

    #[test]
    fn test_open_and_repay_loan() {
        let lpp_balance = 10_000_000;
        let amount = 5_000_000;
        let annual_interest_rate = Percent::from_permille(
            (20 * 1000 * (lpp_balance - amount) / lpp_balance / 700 + 70) as Units,
        );

        let mut deps = testing::mock_dependencies_with_balance(&[coin_cw(lpp_balance as u128)]);
        let mut env = testing::mock_env();
        let admin = Addr::unchecked("admin");
        let loan = Addr::unchecked("loan");
        env.block.time = Timestamp::from_nanos(0);
        let lease_code_id = Uint64::new(123);

        SingleUserAccess::new_contract_owner(admin)
            .store(deps.as_mut().storage)
            .unwrap();

        Config::new(
            TheCurrency::TICKER.into(),
            lease_code_id,
            InterestRate::new(
                BASE_INTEREST_RATE,
                UTILIZATION_OPTIMAL,
                ADDON_OPTIMAL_INTEREST_RATE,
            )
            .expect("Couldn't construct interest rate value!"),
        )
        .store(deps.as_mut().storage)
        .expect("Failed to store Config!");
        Total::<TheCurrency>::new()
            .store(deps.as_mut().storage)
            .expect("can't initialize Total");

        let mut lpp = LiquidityPool::<TheCurrency>::load(deps.as_mut().storage)
            .expect("can't load LiquidityPool");

        // doesn't exist
        let loan_response = lpp
            .query_loan(deps.as_ref().storage, &env, loan.clone())
            .expect("can't query loan");
        assert_eq!(loan_response, None);

        env.block.time = Timestamp::from_nanos(10);

        lpp.try_open_loan(&mut deps.as_mut(), &env, loan.clone(), Coin::new(5_000_000))
            .expect("can't open loan");
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, vec![coin_cw(5_000_000)]);

        let loan_response = lpp
            .query_loan(deps.as_ref().storage, &env, loan.clone())
            .expect("can't query loan")
            .expect("should be some response");

        assert_eq!(loan_response.principal_due, amount.into());
        assert_eq!(loan_response.annual_interest_rate, annual_interest_rate);
        assert_eq!(loan_response.interest_paid, env.block.time);
        assert_eq!(loan_response.interest_due, 0u128.into());

        // wait for year/10
        env.block.time = Timestamp::from_nanos(10 + Duration::YEAR.nanos() / 10);

        // pay interest for year/10
        let payment = lpp
            .query_loan_outstanding_interest(deps.as_ref().storage, loan.clone(), env.block.time)
            .expect("can't query outstanding interest")
            .expect("should be some coins")
            .0;

        let repay = lpp
            .try_repay_loan(&mut deps.as_mut(), &env, loan.clone(), payment)
            .expect("can't repay loan");

        assert_eq!(repay, 0u128.into());

        let loan_response = lpp
            .query_loan(deps.as_ref().storage, &env, loan.clone())
            .expect("can't query loan")
            .expect("should be some response");

        assert_eq!(loan_response.principal_due, amount.into());
        assert_eq!(loan_response.annual_interest_rate, annual_interest_rate);
        assert_eq!(loan_response.interest_paid, env.block.time);
        assert_eq!(loan_response.interest_due, 0u128.into());

        // an immediate repay after repay should pass (loan_interest_due==0 bug)
        lpp.try_repay_loan(&mut deps.as_mut(), &env, loan.clone(), Coin::new(0))
            .expect("can't repay loan");

        // wait for another year/10
        env.block.time = Timestamp::from_nanos(10 + 2 * Duration::YEAR.nanos() / 10);

        // pay everything + excess
        let payment = lpp
            .query_loan_outstanding_interest(deps.as_ref().storage, loan.clone(), env.block.time)
            .expect("can't query outstanding interest")
            .expect("should be some coins")
            .0
            + Coin::new(amount)
            + Coin::new(100);

        let repay = lpp
            .try_repay_loan(&mut deps.as_mut(), &env, loan, payment)
            .expect("can't repay loan");

        assert_eq!(repay, 100u128.into());
    }

    #[test]
    fn try_open_loan_with_no_liquidity() {
        let mut deps = testing::mock_dependencies();
        let env = testing::mock_env();
        let admin = Addr::unchecked("admin");
        let loan = Addr::unchecked("loan");
        let lease_code_id = Uint64::new(123);

        SingleUserAccess::new_contract_owner(admin)
            .store(deps.as_mut().storage)
            .unwrap();

        Config::new(
            TheCurrency::TICKER.into(),
            lease_code_id,
            InterestRate::new(
                BASE_INTEREST_RATE,
                UTILIZATION_OPTIMAL,
                ADDON_OPTIMAL_INTEREST_RATE,
            )
            .expect("Couldn't construct interest rate value!"),
        )
        .store(deps.as_mut().storage)
        .expect("Failed to store Config!");
        Total::<TheCurrency>::new()
            .store(deps.as_mut().storage)
            .expect("can't initialize Total");

        let mut lpp = LiquidityPool::<TheCurrency>::load(deps.as_mut().storage)
            .expect("can't load LiquidityPool");

        let result = lpp.try_open_loan(&mut deps.as_mut(), &env, loan, Coin::new(1_000));
        assert_eq!(result, Err(ContractError::NoLiquidity {}));
    }

    #[test]
    fn try_open_loan_for_zero_amount() {
        let balance_mock = [coin_cw(10_000_000)];
        let mut deps = testing::mock_dependencies_with_balance(&balance_mock);
        let env = testing::mock_env();
        let admin = Addr::unchecked("admin");
        let loan = Addr::unchecked("loan");
        let lease_code_id = Uint64::new(123);

        SingleUserAccess::new_contract_owner(admin)
            .store(deps.as_mut().storage)
            .unwrap();

        Config::new(
            TheCurrency::TICKER.into(),
            lease_code_id,
            InterestRate::new(
                BASE_INTEREST_RATE,
                UTILIZATION_OPTIMAL,
                ADDON_OPTIMAL_INTEREST_RATE,
            )
            .expect("Couldn't construct interest rate value!"),
        )
        .store(deps.as_mut().storage)
        .expect("Failed to store Config!");
        Total::<TheCurrency>::new()
            .store(deps.as_mut().storage)
            .expect("can't initialize Total");

        let mut lpp = LiquidityPool::<TheCurrency>::load(deps.as_mut().storage)
            .expect("can't load LiquidityPool");

        let result = lpp.try_open_loan(&mut deps.as_mut(), &env, loan, Coin::new(0));
        assert_eq!(result, Err(ContractError::ZeroLoanAmount));
    }

    #[test]
    fn open_loan_repay_zero() {
        let balance_mock = [coin_cw(10_000_000)];
        let mut deps = testing::mock_dependencies_with_balance(&balance_mock);
        let env = testing::mock_env();
        let admin = Addr::unchecked("admin");
        let loan = Addr::unchecked("loan");
        let lease_code_id = Uint64::new(123);

        SingleUserAccess::new_contract_owner(admin)
            .store(deps.as_mut().storage)
            .unwrap();

        Config::new(
            TheCurrency::TICKER.into(),
            lease_code_id,
            InterestRate::new(
                BASE_INTEREST_RATE,
                UTILIZATION_OPTIMAL,
                ADDON_OPTIMAL_INTEREST_RATE,
            )
            .expect("Couldn't construct interest rate value!"),
        )
        .store(deps.as_mut().storage)
        .expect("Failed to store Config!");
        Total::<TheCurrency>::new()
            .store(deps.as_mut().storage)
            .expect("can't initialize Total");

        let mut lpp = LiquidityPool::<TheCurrency>::load(deps.as_mut().storage)
            .expect("can't load LiquidityPool");

        lpp.try_open_loan(&mut deps.as_mut(), &env, loan.clone(), Coin::new(5_000))
            .expect("can't open loan");
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, vec![coin_cw(5_000)]);

        let loan_response_before = lpp
            .query_loan(deps.as_ref().storage, &env, loan.clone())
            .expect("can't query loan")
            .expect("should be some response");

        //zero repay
        lpp.try_repay_loan(&mut deps.as_mut(), &env, loan.clone(), Coin::new(0))
            .expect("can't repay loan");

        let loan_response_after = lpp
            .query_loan(deps.as_ref().storage, &env, loan)
            .expect("can't query loan")
            .expect("should be some response");

        //should not change after zero repay
        assert_eq!(
            loan_response_before.principal_due,
            loan_response_after.principal_due
        );
        assert_eq!(
            loan_response_before.annual_interest_rate,
            loan_response_after.annual_interest_rate
        );
        assert_eq!(
            loan_response_before.interest_paid,
            loan_response_after.interest_paid
        );
        assert_eq!(
            loan_response_before.interest_due,
            loan_response_after.interest_due
        );
    }

    #[test]
    fn try_open_and_close_loan_without_paying_interest() {
        let balance_mock = [coin_cw(10_000_000)];
        let mut deps = testing::mock_dependencies_with_balance(&balance_mock);
        let env = testing::mock_env();
        let admin = Addr::unchecked("admin");
        let loan = Addr::unchecked("loan");
        let lease_code_id = Uint64::new(123);

        SingleUserAccess::new_contract_owner(admin)
            .store(deps.as_mut().storage)
            .unwrap();

        Config::new(
            TheCurrency::TICKER.into(),
            lease_code_id,
            InterestRate::new(
                BASE_INTEREST_RATE,
                UTILIZATION_OPTIMAL,
                ADDON_OPTIMAL_INTEREST_RATE,
            )
            .expect("Couldn't construct interest rate value!"),
        )
        .store(deps.as_mut().storage)
        .expect("Failed to store Config!");
        Total::<TheCurrency>::new()
            .store(deps.as_mut().storage)
            .expect("can't initialize Total");

        let mut lpp = LiquidityPool::<TheCurrency>::load(deps.as_mut().storage)
            .expect("can't load LiquidityPool");

        lpp.try_open_loan(&mut deps.as_mut(), &env, loan.clone(), Coin::new(5_000))
            .expect("can't open loan");
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, vec![coin_cw(5_000)]);

        let payment = lpp
            .query_loan_outstanding_interest(deps.as_ref().storage, loan.clone(), env.block.time)
            .expect("can't query outstanding interest")
            .expect("should be some coins")
            .0;
        assert_eq!(payment, Coin::new(0));

        let repay = lpp
            .try_repay_loan(&mut deps.as_mut(), &env, loan.clone(), Coin::new(5_000))
            .expect("can't repay loan");

        assert_eq!(repay, 0u128.into());

        // Should be closed
        let loan_response = lpp
            .query_loan(deps.as_ref().storage, &env, loan)
            .expect("can't query loan");
        assert_eq!(loan_response, None);
    }

    #[test]
    fn test_tvl_and_price() {
        let balance_mock = coin_cw(0); // will deposit something later
        let mut deps = testing::mock_dependencies_with_balance(&[balance_mock.clone()]);
        let mut env = testing::mock_env();
        let admin = Addr::unchecked("admin");
        let loan = Addr::unchecked("loan");
        env.block.time = Timestamp::from_nanos(0);
        let lease_code_id = Uint64::new(123);

        SingleUserAccess::new_contract_owner(admin)
            .store(deps.as_mut().storage)
            .unwrap();

        Config::new(
            balance_mock.denom,
            lease_code_id,
            InterestRate::new(
                BASE_INTEREST_RATE,
                UTILIZATION_OPTIMAL,
                ADDON_OPTIMAL_INTEREST_RATE,
            )
            .expect("Couldn't construct interest rate value!"),
        )
        .store(deps.as_mut().storage)
        .expect("Failed to store Config!");

        // simplify calculation
        Config::update_borrow_rate(
            deps.as_mut().storage,
            InterestRate::new(
                Percent::from_percent(18),
                Percent::from_percent(50),
                Percent::from_percent(2),
            )
            .expect("Couldn't construct interest rate value!"),
        )
        .expect("should update config");

        Total::<TheCurrency>::new()
            .store(deps.as_mut().storage)
            .expect("can't initialize Total");

        let mut lpp = LiquidityPool::<TheCurrency>::load(deps.as_mut().storage)
            .expect("can't load LiquidityPool");

        let mut lender = Deposit::load_or_default(deps.as_ref().storage, Addr::unchecked("lender"))
            .expect("should load");
        let price = lpp
            .calculate_price(&deps.as_ref(), &env, Coin::new(0))
            .expect("should get price");
        assert_eq!(price.get(), Price::identity());

        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, vec![coin_cw(10_000_000)]);
        lender
            .deposit(deps.as_mut().storage, 10_000_000u128.into(), price)
            .expect("should deposit");

        let annual_interest_rate = lpp
            .query_quote(&deps.as_ref(), &env, Coin::new(5_000_000))
            .expect("can't query quote")
            .expect("should return some interest_rate");

        assert_eq!(annual_interest_rate, Percent::from_percent(20));

        lpp.try_open_loan(&mut deps.as_mut(), &env, loan.clone(), Coin::new(5_000_000))
            .expect("can't open loan");
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, vec![coin_cw(5_000_000)]);

        // wait a year
        env.block.time = Timestamp::from_nanos(Duration::YEAR.nanos());

        let total_lpn = lpp
            .total_lpn(&deps.as_ref(), &env)
            .expect("should query total_lpn");
        assert_eq!(total_lpn, 11_000_000u128.into());

        let lpp_balance = lpp
            .query_lpp_balance(&deps.as_ref(), &env)
            .expect("should query_lpp_balance");
        assert_eq!(lpp_balance.balance, Coin::new(5_000_000));
        assert_eq!(lpp_balance.total_principal_due, Coin::new(5_000_000));
        assert_eq!(lpp_balance.total_interest_due, Coin::new(1_000_000));

        let price = lpp
            .calculate_price(&deps.as_ref(), &env, Coin::new(0))
            .expect("should get price");
        assert_eq!(
            price::total(Coin::<NLpn>::new(1000), price.get()),
            price::total(
                Coin::<NLpn>::new(1000),
                price::total_of(Coin::new(10)).is(Coin::new(11))
            )
        );

        // should not change tvl/price
        let excess = lpp
            .try_repay_loan(&mut deps.as_mut(), &env, loan, Coin::new(6_000_000))
            .unwrap();
        assert_eq!(excess, Coin::new(0));

        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, vec![coin_cw(11_000_000)]);
        let total_lpn = lpp
            .total_lpn(&deps.as_ref(), &env)
            .expect("should query total_lpn");
        assert_eq!(total_lpn, 11_000_000u128.into());

        let price = lpp
            .calculate_price(&deps.as_ref(), &env, Coin::new(0))
            .expect("should get price");
        assert_eq!(
            price::total(Coin::<NLpn>::new(1000), price.get()),
            price::total(
                Coin::<NLpn>::new(1000),
                price::total_of(Coin::new(10)).is(Coin::new(11))
            )
        );

        let withdraw = lpp
            .withdraw_lpn(&deps.as_ref(), &env, 1000u128.into())
            .expect("should withdraw");
        assert_eq!(withdraw, Coin::new(1100));
    }

    fn coin_cw(amount: u128) -> CwCoin {
        coin_legacy::to_cosmwasm::<TheCurrency>(amount.into())
    }
}
