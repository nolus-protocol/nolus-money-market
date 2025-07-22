use currencies::Lpns;
use currency::{CurrencyDef, MemberOf};
use finance::{
    coin::Coin,
    fraction::Fraction,
    percent::{Percent, Units},
    price,
    ratio::Rational,
    zero::Zero,
};
use lpp_platform::NLpn;
use platform::{bank, contract};
use sdk::cosmwasm_std::{Addr, Deps, DepsMut, Env, QuerierWrapper, Storage, Timestamp};

use crate::{
    config::Config as ApiConfig,
    contract::{ContractError, Result},
    loan::Loan,
    loans::Repo,
    msg::LppBalanceResponse,
    nprice::NTokenPrice,
    state::{Config, Deposit, Total},
};

pub struct LppBalances<Lpn> {
    pub(crate) balance: Coin<Lpn>,
    pub(crate) total_principal_due: Coin<Lpn>,
    pub(crate) total_interest_due: Coin<Lpn>,
}

impl<Lpn> LppBalances<Lpn> {
    pub(crate) fn into_total(self) -> Coin<Lpn> {
        self.balance + self.total_principal_due + self.total_interest_due
    }
}

impl<Lpn> LppBalances<Lpn>
where
    Lpn: CurrencyDef,
    Lpn::Group: MemberOf<Lpns>,
{
    pub(crate) fn into_response(self, total_rewards: Coin<NLpn>) -> LppBalanceResponse<Lpns> {
        LppBalanceResponse {
            balance: self.balance.into(),
            total_principal_due: self.total_principal_due.into(),
            total_interest_due: self.total_interest_due.into(),
            balance_nlpn: total_rewards,
        }
    }
}

// TODO reverse the direction of the dependencies between LiquidityPool and Deposit,
// and LiquidityPool and Loan. The contract API implementation should depend on
// Deposit and Loan which in turn may use LiquidityPool.

pub(crate) struct LiquidityPool<Lpn> {
    config: ApiConfig,
    total: Total<Lpn>,
}

impl<Lpn> LiquidityPool<Lpn>
where
    Lpn: 'static,
{
    pub fn initialize(storage: &mut dyn Storage, config: &ApiConfig) -> Result<()> {
        Config::store(config, storage)
            .and_then(|()| Total::<Lpn>::new().store(storage).map_err(Into::into))
    }

    pub fn load(storage: &dyn Storage) -> Result<Self> {
        let config = Config::load(storage)?;
        let total = Total::load(storage)?;

        Ok(LiquidityPool { config, total })
    }
}

impl<Lpn> LiquidityPool<Lpn>
where
    Lpn: 'static + CurrencyDef,
{
    pub fn deposit_capacity(
        &self,
        querier: QuerierWrapper<'_>,
        env: &Env,
        pending_deposit: Coin<Lpn>,
    ) -> Result<Option<Coin<Lpn>>> {
        let min_utilization: Percent = self.config.min_utilization().percent();

        if min_utilization.is_zero() {
            Ok(None)
        } else {
            let total_due: Coin<Lpn> = self.total_due(&env.block.time);

            self.commited_balance(&env.contract.address, querier, pending_deposit)
                .map(|balance: Coin<Lpn>| {
                    if self.utilization(balance, total_due) > min_utilization {
                        // a followup from the above true value is (total_due * 100 / min_utilization) > (balance + total_due)
                        Fraction::<Units>::of(
                            &Rational::new(Percent::HUNDRED, min_utilization),
                            total_due,
                        ) - balance
                            - total_due
                    } else {
                        Coin::ZERO
                    }
                })
                .map(Some)
        }
    }

    pub fn query_lpp_balance(&self, deps: &Deps<'_>, env: &Env) -> Result<LppBalances<Lpn>> {
        let balance = self.balance(&env.contract.address, deps.querier)?;

        let total_principal_due = self.total.total_principal_due();

        let total_interest_due = self.total.total_interest_due_by_now(&env.block.time);

        Ok(LppBalances {
            balance,
            total_principal_due,
            total_interest_due,
        })
    }

    pub fn calculate_price(
        &self,
        deps: &Deps<'_>,
        env: &Env,
        pending_deposit: Coin<Lpn>,
    ) -> Result<NTokenPrice<Lpn>> {
        let balance_nlpn = Deposit::balance_nlpn(deps.storage)?;

        let price = if balance_nlpn.is_zero() {
            ApiConfig::initial_derivative_price()
        } else {
            price::total_of(balance_nlpn).is(self.total_lpn(
                deps.querier,
                &env.contract.address,
                &env.block.time,
                pending_deposit,
            )?)
        };

        debug_assert!(
            price >= ApiConfig::initial_derivative_price::<Lpn>(),
            "[Lpp] programming error: nlpn price less than initial"
        );

        Ok(price)
    }

    pub fn validate_lease_addr(&self, deps: &Deps<'_>, lease_addr: &Addr) -> Result<()> {
        contract::validate_code_id(deps.querier, lease_addr, self.config.lease_code())
            .map_err(ContractError::from)
    }

    pub fn withdraw_lpn(
        &self,
        deps: &Deps<'_>,
        env: &Env,
        amount_nlpn: Coin<NLpn>,
    ) -> Result<Coin<Lpn>> {
        let price = self.calculate_price(deps, env, Coin::ZERO)?;
        let amount_lpn = price::total(amount_nlpn, price);

        if self.balance(&env.contract.address, deps.querier)? < amount_lpn {
            Err(ContractError::NoLiquidity {})
        } else {
            Ok(amount_lpn)
        }
    }

    pub fn query_quote(
        &self,
        quote: Coin<Lpn>,
        account: &Addr,
        querier: QuerierWrapper<'_>,
        now: &Timestamp,
    ) -> Result<Option<Percent>> {
        let balance = self.balance(account, querier)?;

        if quote > balance {
            return Ok(None);
        }

        let total_principal_due = self.total.total_principal_due();
        let total_interest = self.total.total_interest_due_by_now(now);
        let total_liability_past_quote = total_principal_due + quote + total_interest;
        let total_balance_past_quote = balance - quote;

        Ok(Some(self.config.borrow_rate().calculate(
            total_liability_past_quote,
            total_balance_past_quote,
        )))
    }

    pub(super) fn try_open_loan(
        &mut self,
        deps: &mut DepsMut<'_>,
        env: &Env,
        lease_addr: Addr,
        amount: Coin<Lpn>,
    ) -> Result<Loan<Lpn>> {
        if amount.is_zero() {
            return Err(ContractError::ZeroLoanAmount);
        }

        let now = env.block.time;

        let annual_interest_rate =
            match self.query_quote(amount, &env.contract.address, deps.querier, &now)? {
                Some(rate) => Ok(rate),
                None => Err(ContractError::NoLiquidity {}),
            }?;

        let loan = Loan {
            principal_due: amount,
            annual_interest_rate,
            interest_paid: now,
        };

        Repo::open(deps.storage, lease_addr, &loan)?;

        self.total
            .borrow(now, amount, annual_interest_rate)?
            .store(deps.storage)?;

        Ok(loan)
    }

    /// return amount of lpp currency to pay back to lease_addr
    pub(super) fn try_repay_loan(
        &mut self,
        deps: &mut DepsMut<'_>,
        env: &Env,
        lease_addr: Addr,
        repay_amount: Coin<Lpn>,
    ) -> Result<Coin<Lpn>> {
        let mut loan = Repo::load(deps.storage, lease_addr.clone())?;
        let loan_annual_interest_rate = loan.annual_interest_rate;
        let payment = loan.repay(&env.block.time, repay_amount);
        Repo::save(deps.storage, lease_addr, loan)?;

        self.total
            .repay(
                env.block.time,
                payment.interest,
                payment.principal,
                loan_annual_interest_rate,
            )
            .store(deps.storage)?;

        Ok(payment.excess)
    }

    fn balance(&self, account: &Addr, querier: QuerierWrapper<'_>) -> Result<Coin<Lpn>> {
        self.uncommited_balance(account, querier)
    }

    fn commited_balance(
        &self,
        account: &Addr,
        querier: QuerierWrapper<'_>,
        pending_deposit: Coin<Lpn>,
    ) -> Result<Coin<Lpn>> {
        self.uncommited_balance(account, querier)
            .map(|balance: Coin<Lpn>| {
                debug_assert!(
                    pending_deposit <= balance,
                    "Pending deposit {{{pending_deposit:?}}} > Current Balance: {{{balance}}}!"
                );
                balance - pending_deposit
            })
    }

    fn uncommited_balance(&self, account: &Addr, querier: QuerierWrapper<'_>) -> Result<Coin<Lpn>> {
        bank::balance(account, querier).map_err(Into::into)
    }

    fn total_due(&self, now: &Timestamp) -> Coin<Lpn> {
        self.total.total_principal_due() + self.total.total_interest_due_by_now(now)
    }

    fn total_lpn(
        &self,
        querier: QuerierWrapper<'_>,
        account: &Addr,
        now: &Timestamp,
        pending_deposit: Coin<Lpn>,
    ) -> Result<Coin<Lpn>> {
        self.commited_balance(account, querier, pending_deposit)
            .map(|balance: Coin<Lpn>| balance + self.total_due(now))
    }

    fn utilization(&self, balance: Coin<Lpn>, total_due: Coin<Lpn>) -> Percent {
        if balance.is_zero() {
            Percent::HUNDRED
        } else {
            Percent::from_ratio(total_due, total_due + balance)
        }
    }
}

#[cfg(test)]
mod test {
    use access_control::ContractOwnerAccess;
    use currencies::Lpn;
    use finance::{
        coin::{Amount, Coin},
        duration::Duration,
        percent::{Percent, bound::BoundToHundredPercent},
        price::{self, Price},
        zero::Zero,
    };
    use lpp_platform::NLpn;
    use platform::{coin_legacy, contract::Code};
    use sdk::cosmwasm_std::{
        Addr, Coin as CwCoin, DepsMut, Timestamp,
        testing::{self, MOCK_CONTRACT_ADDR},
    };

    use crate::{
        borrow::InterestRate,
        config::Config as ApiConfig,
        contract::ContractError,
        loans::Repo,
        state::{Config, Deposit, Total},
    };

    use super::LiquidityPool;

    type TheCurrency = Lpn;

    const BASE_INTEREST_RATE: Percent = Percent::from_permille(70);
    const UTILIZATION_OPTIMAL: Percent = Percent::from_permille(700);
    const ADDON_OPTIMAL_INTEREST_RATE: Percent = Percent::from_permille(20);
    const DEFAULT_MIN_UTILIZATION: BoundToHundredPercent = BoundToHundredPercent::ZERO;

    #[test]
    fn test_balance() {
        let balance_mock = coin_cw(10_000_000);
        let mut deps = testing::mock_dependencies_with_balance(&[balance_mock.clone()]);
        let env = testing::mock_env();
        let lease_code_id = Code::unchecked(123);
        let admin = Addr::unchecked("admin");

        grant_admin_access(deps.as_mut(), &admin);

        Config::store(
            &ApiConfig::new(
                lease_code_id,
                InterestRate::new(
                    BASE_INTEREST_RATE,
                    UTILIZATION_OPTIMAL,
                    ADDON_OPTIMAL_INTEREST_RATE,
                )
                .expect("Couldn't construct interest rate value!"),
                DEFAULT_MIN_UTILIZATION,
            ),
            deps.as_mut().storage,
        )
        .expect("Failed to store Config!");
        Total::<TheCurrency>::new()
            .store(deps.as_mut().storage)
            .expect("can't initialize Total");

        let lpp = LiquidityPool::<TheCurrency>::load(deps.as_mut().storage)
            .expect("can't load LiquidityPool");

        let balance = lpp
            .balance(&env.contract.address, deps.as_ref().querier)
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

        let lease_code_id = Code::unchecked(123);

        grant_admin_access(deps.as_mut(), &admin);

        Config::store(
            &ApiConfig::new(
                lease_code_id,
                InterestRate::new(
                    BASE_INTEREST_RATE,
                    UTILIZATION_OPTIMAL,
                    ADDON_OPTIMAL_INTEREST_RATE,
                )
                .expect("Couldn't construct interest rate value!"),
                DEFAULT_MIN_UTILIZATION,
            ),
            deps.as_mut().storage,
        )
        .expect("Failed to store Config!");
        Total::<TheCurrency>::new()
            .store(deps.as_mut().storage)
            .expect("can't initialize Total");

        let mut lpp = LiquidityPool::<TheCurrency>::load(deps.as_mut().storage)
            .expect("can't load LiquidityPool");

        env.block.time = Timestamp::from_nanos(10);

        let result = lpp
            .query_quote(
                Coin::new(7_700_000),
                &env.contract.address,
                deps.as_ref().querier,
                &env.block.time,
            )
            .expect("can't query quote")
            .expect("should return some interest_rate");

        assert_eq!(result, Percent::from_permille(136));

        lpp.try_open_loan(&mut deps.as_mut(), &env, loan, Coin::new(7_000_000))
            .expect("can't open loan");
        deps.querier
            .bank
            .update_balance(MOCK_CONTRACT_ADDR, vec![coin_cw(3_000_000)]);

        // wait for a year
        env.block.time = Timestamp::from_nanos(10 + Duration::YEAR.nanos());

        let result = lpp
            .query_quote(
                Coin::new(1_000_000),
                &env.contract.address,
                deps.as_ref().querier,
                &env.block.time,
            )
            .expect("can't query quote")
            .expect("should return some interest_rate");

        assert_eq!(result, Percent::from_permille(136));
    }

    #[test]
    fn test_open_and_repay_loan() {
        let lpp_balance: Amount = 10_000_000;
        let amount = 5_000_000;

        let base_rate = 70;
        let addon_rate = 20;
        let optimal_rate = 700;

        let utilization_const = (optimal_rate * 1000) / (1000 - optimal_rate);
        let utilization_relative = ((lpp_balance - amount) * 1000) / amount;
        let utilization = utilization_relative.min(utilization_const);

        let annual_interest_rate = Percent::from_permille(
            (base_rate + ((utilization * addon_rate) / optimal_rate))
                .try_into()
                .unwrap(),
        );

        let mut deps = testing::mock_dependencies_with_balance(&[coin_cw(lpp_balance)]);
        let mut env = testing::mock_env();
        let admin = Addr::unchecked("admin");
        let lease_addr = Addr::unchecked("loan");
        env.block.time = Timestamp::from_nanos(0);
        let lease_code_id = Code::unchecked(123);

        grant_admin_access(deps.as_mut(), &admin);

        Config::store(
            &ApiConfig::new(
                lease_code_id,
                InterestRate::new(
                    BASE_INTEREST_RATE,
                    UTILIZATION_OPTIMAL,
                    ADDON_OPTIMAL_INTEREST_RATE,
                )
                .expect("Couldn't construct interest rate value!"),
                DEFAULT_MIN_UTILIZATION,
            ),
            deps.as_mut().storage,
        )
        .expect("Failed to store Config!");
        Total::<TheCurrency>::new()
            .store(deps.as_mut().storage)
            .expect("can't initialize Total");

        let mut lpp = LiquidityPool::<TheCurrency>::load(deps.as_mut().storage)
            .expect("can't load LiquidityPool");

        // doesn't exist
        let loan_response = Repo::<TheCurrency>::query(deps.as_ref().storage, lease_addr.clone())
            .expect("can't query loan");
        assert_eq!(loan_response, None);

        env.block.time = Timestamp::from_nanos(10);

        lpp.try_open_loan(
            &mut deps.as_mut(),
            &env,
            lease_addr.clone(),
            Coin::new(5_000_000),
        )
        .expect("can't open loan");
        deps.querier
            .bank
            .update_balance(MOCK_CONTRACT_ADDR, vec![coin_cw(5_000_000)]);

        let loan = Repo::query(deps.as_ref().storage, lease_addr.clone())
            .expect("can't query loan")
            .expect("should be some response");

        assert_eq!(loan.principal_due, Coin::new(amount));
        assert_eq!(loan.annual_interest_rate, annual_interest_rate);
        assert_eq!(loan.interest_paid, env.block.time);
        assert_eq!(loan.interest_due(&env.block.time), 0u128.into());

        // wait for year/10
        env.block.time = Timestamp::from_nanos(10 + Duration::YEAR.nanos() / 10);

        // pay interest for year/10
        let payment = loan.interest_due(&env.block.time);

        let repay = lpp
            .try_repay_loan(&mut deps.as_mut(), &env, lease_addr.clone(), payment)
            .expect("can't repay loan");

        assert_eq!(repay, 0u128.into());

        let loan = Repo::<TheCurrency>::query(deps.as_ref().storage, lease_addr.clone())
            .expect("can't query loan")
            .expect("should be some response");

        assert_eq!(loan.principal_due, Coin::new(amount));
        assert_eq!(loan.annual_interest_rate, annual_interest_rate);
        assert_eq!(loan.interest_paid, env.block.time);
        assert_eq!(loan.interest_due(&env.block.time), 0u128.into());

        // an immediate repay after repay should pass (loan_interest_due==0 bug)
        lpp.try_repay_loan(&mut deps.as_mut(), &env, lease_addr.clone(), Coin::new(0))
            .expect("can't repay loan");

        // wait for another year/10
        env.block.time = Timestamp::from_nanos(10 + 2 * Duration::YEAR.nanos() / 10);

        // pay everything + excess
        let payment = Repo::query(deps.as_ref().storage, lease_addr.clone())
            .expect("can't query the loan")
            .expect("should exist")
            .interest_due(&env.block.time)
            + Coin::new(amount)
            + Coin::new(100);

        let repay = lpp
            .try_repay_loan(&mut deps.as_mut(), &env, lease_addr, payment)
            .expect("can't repay loan");

        assert_eq!(repay, 100u128.into());
    }

    #[test]
    fn try_open_loan_with_no_liquidity() {
        let mut deps = testing::mock_dependencies();
        let env = testing::mock_env();
        let admin = Addr::unchecked("admin");
        let loan = Addr::unchecked("loan");
        let lease_code_id = Code::unchecked(123);

        grant_admin_access(deps.as_mut(), &admin);
        Config::store(
            &ApiConfig::new(
                lease_code_id,
                InterestRate::new(
                    BASE_INTEREST_RATE,
                    UTILIZATION_OPTIMAL,
                    ADDON_OPTIMAL_INTEREST_RATE,
                )
                .expect("Couldn't construct interest rate value!"),
                DEFAULT_MIN_UTILIZATION,
            ),
            deps.as_mut().storage,
        )
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
        let lease_code_id = Code::unchecked(123);

        grant_admin_access(deps.as_mut(), &admin);
        Config::store(
            &ApiConfig::new(
                lease_code_id,
                InterestRate::new(
                    BASE_INTEREST_RATE,
                    UTILIZATION_OPTIMAL,
                    ADDON_OPTIMAL_INTEREST_RATE,
                )
                .expect("Couldn't construct interest rate value!"),
                DEFAULT_MIN_UTILIZATION,
            ),
            deps.as_mut().storage,
        )
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
        let lease_code_id = Code::unchecked(123);

        grant_admin_access(deps.as_mut(), &admin);
        Config::store(
            &ApiConfig::new(
                lease_code_id,
                InterestRate::new(
                    BASE_INTEREST_RATE,
                    UTILIZATION_OPTIMAL,
                    ADDON_OPTIMAL_INTEREST_RATE,
                )
                .expect("Couldn't construct interest rate value!"),
                DEFAULT_MIN_UTILIZATION,
            ),
            deps.as_mut().storage,
        )
        .expect("Failed to store Config!");
        Total::<TheCurrency>::new()
            .store(deps.as_mut().storage)
            .expect("can't initialize Total");

        let mut lpp = LiquidityPool::<TheCurrency>::load(deps.as_mut().storage)
            .expect("can't load LiquidityPool");

        lpp.try_open_loan(&mut deps.as_mut(), &env, loan.clone(), Coin::new(5_000))
            .expect("can't open loan");
        deps.querier
            .bank
            .update_balance(MOCK_CONTRACT_ADDR, vec![coin_cw(5_000)]);

        let loan_before = Repo::<TheCurrency>::query(deps.as_ref().storage, loan.clone())
            .expect("can't query loan")
            .expect("should be some response");

        //zero repay
        lpp.try_repay_loan(&mut deps.as_mut(), &env, loan.clone(), Coin::new(0))
            .expect("can't repay loan");

        let loan_after = Repo::query(deps.as_ref().storage, loan)
            .expect("can't query loan")
            .expect("should be some response");

        //should not change after zero repay
        assert_eq!(loan_before.principal_due, loan_after.principal_due);
        assert_eq!(
            loan_before.annual_interest_rate,
            loan_after.annual_interest_rate
        );
        assert_eq!(loan_before.interest_paid, loan_after.interest_paid);
    }

    #[test]
    fn try_open_and_close_loan_without_paying_interest() {
        let balance_mock = [coin_cw(10_000_000)];
        let mut deps = testing::mock_dependencies_with_balance(&balance_mock);
        let env = testing::mock_env();
        let admin = Addr::unchecked("admin");
        let loan = Addr::unchecked("loan");
        let lease_code_id = Code::unchecked(123);

        grant_admin_access(deps.as_mut(), &admin);
        Config::store(
            &ApiConfig::new(
                lease_code_id,
                InterestRate::new(
                    BASE_INTEREST_RATE,
                    UTILIZATION_OPTIMAL,
                    ADDON_OPTIMAL_INTEREST_RATE,
                )
                .expect("Couldn't construct interest rate value!"),
                DEFAULT_MIN_UTILIZATION,
            ),
            deps.as_mut().storage,
        )
        .expect("Failed to store Config!");
        Total::<TheCurrency>::new()
            .store(deps.as_mut().storage)
            .expect("can't initialize Total");

        let mut lpp = LiquidityPool::<TheCurrency>::load(deps.as_mut().storage)
            .expect("can't load LiquidityPool");

        lpp.try_open_loan(&mut deps.as_mut(), &env, loan.clone(), Coin::new(5_000))
            .expect("can't open loan");
        deps.querier
            .bank
            .update_balance(MOCK_CONTRACT_ADDR, vec![coin_cw(5_000)]);

        let payment = Repo::<TheCurrency>::query(deps.as_ref().storage, loan.clone())
            .expect("can't query outstanding interest")
            .expect("should be some coins")
            .interest_due(&env.block.time);
        assert_eq!(payment, Coin::new(0));

        let repay = lpp
            .try_repay_loan(&mut deps.as_mut(), &env, loan.clone(), Coin::new(5_000))
            .expect("can't repay loan");

        assert_eq!(repay, 0u128.into());

        // Should be closed
        let loan_response =
            Repo::<TheCurrency>::query(deps.as_ref().storage, loan).expect("can't query loan");
        assert_eq!(loan_response, None);
    }

    #[test]
    fn test_tvl_and_price() {
        let mut deps = testing::mock_dependencies_with_balance(&[]);
        let mut env = testing::mock_env();
        let admin = Addr::unchecked("admin");
        let loan = Addr::unchecked("loan");
        env.block.time = Timestamp::from_nanos(0);
        let lease_code_id = Code::unchecked(123);

        grant_admin_access(deps.as_mut(), &admin);
        Config::store(
            &ApiConfig::new(
                lease_code_id,
                InterestRate::new(
                    BASE_INTEREST_RATE,
                    UTILIZATION_OPTIMAL,
                    ADDON_OPTIMAL_INTEREST_RATE,
                )
                .expect("Couldn't construct interest rate value!"),
                DEFAULT_MIN_UTILIZATION,
            ),
            deps.as_mut().storage,
        )
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
        assert_eq!(price, Price::identity());

        deps.querier
            .bank
            .update_balance(MOCK_CONTRACT_ADDR, vec![coin_cw(10_000_000)]);
        lender
            .deposit(deps.as_mut().storage, 10_000_000u128.into())
            .expect("should deposit");

        let annual_interest_rate = lpp
            .query_quote(
                Coin::new(5_000_000),
                &env.contract.address,
                deps.as_ref().querier,
                &env.block.time,
            )
            .expect("can't query quote")
            .expect("should return some interest_rate");

        assert_eq!(annual_interest_rate, Percent::from_permille(220));

        lpp.try_open_loan(&mut deps.as_mut(), &env, loan.clone(), Coin::new(5_000_000))
            .expect("can't open loan");
        deps.querier
            .bank
            .update_balance(MOCK_CONTRACT_ADDR, vec![coin_cw(5_000_000)]);

        // wait a year
        env.block.time = Timestamp::from_nanos(Duration::YEAR.nanos());

        let total_lpn = lpp
            .total_lpn(
                deps.as_ref().querier,
                &env.contract.address,
                &env.block.time,
                Coin::ZERO,
            )
            .expect("should query total_lpn");
        assert_eq!(total_lpn, 11_100_000u128.into());

        let lpp_balance = lpp
            .query_lpp_balance(&deps.as_ref(), &env)
            .expect("should query_lpp_balance");
        assert_eq!(lpp_balance.balance, Coin::<TheCurrency>::new(5_000_000));
        assert_eq!(
            lpp_balance.total_principal_due,
            Coin::<TheCurrency>::new(5_000_000)
        );
        assert_eq!(
            lpp_balance.total_interest_due,
            Coin::<TheCurrency>::new(1_100_000)
        );

        let price = lpp
            .calculate_price(&deps.as_ref(), &env, Coin::new(0))
            .expect("should get price");
        assert_eq!(
            price::total(Coin::<NLpn>::new(1000), price),
            price::total(
                Coin::<NLpn>::new(1000),
                price::total_of(Coin::new(100)).is(Coin::new(111))
            )
        );

        // should not change tvl/price
        let excess = lpp
            .try_repay_loan(&mut deps.as_mut(), &env, loan, Coin::new(6_000_000))
            .unwrap();
        assert_eq!(excess, Coin::new(0));

        deps.querier
            .bank
            .update_balance(MOCK_CONTRACT_ADDR, vec![coin_cw(11_000_000)]);
        let total_lpn = lpp
            .total_lpn(
                deps.as_ref().querier,
                &env.contract.address,
                &env.block.time,
                Coin::ZERO,
            )
            .expect("should query total_lpn");
        assert_eq!(total_lpn, 11_100_000u128.into());

        let price = lpp
            .calculate_price(&deps.as_ref(), &env, Coin::new(0))
            .expect("should get price");
        assert_eq!(
            price::total(Coin::<NLpn>::new(1000), price),
            price::total(
                Coin::<NLpn>::new(1000),
                price::total_of(Coin::new(100)).is(Coin::new(111))
            )
        );

        let withdraw = lpp
            .withdraw_lpn(&deps.as_ref(), &env, 1000u128.into())
            .expect("should withdraw");
        assert_eq!(withdraw, Coin::new(1110));
    }

    fn coin_cw<IntoCoin>(into_coin: IntoCoin) -> CwCoin
    where
        IntoCoin: Into<Coin<TheCurrency>>,
    {
        coin_legacy::to_cosmwasm_on_nolus::<TheCurrency>(into_coin.into())
    }

    fn grant_admin_access(deps: DepsMut<'_>, admin: &Addr) {
        ContractOwnerAccess::new(deps.storage)
            .grant_to(admin)
            .unwrap();
    }

    mod min_utilization {
        use finance::{
            coin::{Amount, Coin},
            percent::{Percent, bound::BoundToHundredPercent},
            zero::Zero,
        };
        use platform::contract::Code;
        use sdk::cosmwasm_std::{
            Env, QuerierWrapper, Timestamp,
            testing::{MockQuerier, mock_env},
        };

        use crate::{borrow::InterestRate, config::Config as ApiConfig, state::Total};

        use super::{super::LiquidityPool, TheCurrency, coin_cw};

        const FIFTY_PERCENT_MIN_UTILIZATION: fn() -> BoundToHundredPercent =
            || Percent::from_permille(500).try_into().unwrap();

        fn test_case(
            borrowed: Amount,
            lpp_balance: Amount,
            min_utilization: BoundToHundredPercent,
            expected_limit: Option<Amount>,
        ) {
            let mut total: Total<TheCurrency> = Total::new();

            total
                .borrow(Timestamp::default(), borrowed.into(), Percent::ZERO)
                .unwrap();

            let lpp: LiquidityPool<TheCurrency> = LiquidityPool {
                config: ApiConfig::new(
                    Code::unchecked(0xDEADC0DE_u64),
                    InterestRate::new(Percent::ZERO, Percent::from_permille(500), Percent::HUNDRED)
                        .unwrap(),
                    min_utilization,
                ),
                total,
            };

            let mock_env: Env = mock_env();
            let mock_querier: MockQuerier =
                MockQuerier::new(&[(mock_env.contract.address.as_str(), &[coin_cw(lpp_balance)])]);
            let mock_querier: QuerierWrapper<'_> = QuerierWrapper::new(&mock_querier);

            assert_eq!(
                lpp.deposit_capacity(mock_querier, &mock_env, Coin::ZERO)
                    .unwrap(),
                expected_limit.map(Into::into)
            );
        }

        #[test]
        fn test_deposit_capacity_no_min_util_below_50() {
            test_case(50, 100, BoundToHundredPercent::ZERO, None);
        }

        #[test]
        fn test_deposit_capacity_no_min_util_at_50() {
            test_case(50, 50, BoundToHundredPercent::ZERO, None);
        }

        #[test]
        fn test_deposit_capacity_no_min_util_above_50() {
            test_case(100, 50, BoundToHundredPercent::ZERO, None);
        }

        #[test]
        fn test_deposit_capacity_no_min_util_at_100() {
            test_case(50, 0, BoundToHundredPercent::ZERO, None);
        }

        #[test]
        fn test_deposit_capacity_below_min_util() {
            test_case(
                50,
                100,
                FIFTY_PERCENT_MIN_UTILIZATION(),
                Some(Default::default()),
            );
        }

        #[test]
        fn test_deposit_capacity_at_min_util() {
            test_case(
                50,
                50,
                FIFTY_PERCENT_MIN_UTILIZATION(),
                Some(Default::default()),
            );
        }

        #[test]
        fn test_deposit_capacity_above_min_util() {
            test_case(100, 50, FIFTY_PERCENT_MIN_UTILIZATION(), Some(50));
        }

        #[test]
        fn test_deposit_capacity_at_max_util() {
            test_case(50, 0, FIFTY_PERCENT_MIN_UTILIZATION(), Some(50));
        }
    }
}
