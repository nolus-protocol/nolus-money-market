use currencies::Lpns;
use currency::{CurrencyDef, MemberOf};
use finance::{
    coin::Coin, percent::Percent100, price, ratio::SimpleFraction, rational::Rational, zero::Zero,
};
use lpp_platform::NLpn;
use platform::{bank::BankAccountView, contract::Validator};
use sdk::cosmwasm_std::{Addr, Storage, Timestamp};

use crate::{
    config::Config as ApiConfig,
    contract::{ContractError, Result},
    loan::{Loan, RepayShares},
    msg::LppBalanceResponse,
    nprice::NTokenPrice,
    state::Total,
};

#[cfg_attr(test, derive(Debug, PartialEq))]
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
    pub(crate) fn into_response(self, balance_receipts: Coin<NLpn>) -> LppBalanceResponse<Lpns> {
        LppBalanceResponse {
            balance: self.balance.into(),
            total_principal_due: self.total_principal_due.into(),
            total_interest_due: self.total_interest_due.into(),
            balance_nlpn: balance_receipts,
        }
    }
}

pub(crate) struct LiquidityPool<'cfg, 'bank, Lpn, Bank> {
    config: &'cfg ApiConfig,
    bank: &'bank Bank,
    total: Total<Lpn>,
}

impl<'cfg, 'bank, Lpn, Bank> LiquidityPool<'cfg, 'bank, Lpn, Bank> {
    pub fn new(config: &'cfg ApiConfig, bank: &'bank Bank) -> Self {
        Self {
            config,
            bank,
            total: Total::new(),
        }
    }

    pub fn save(self, storage: &mut dyn Storage) -> Result<()> {
        self.total.store(storage)
    }

    pub fn load(storage: &dyn Storage, config: &'cfg ApiConfig, bank: &'bank Bank) -> Result<Self> {
        Total::load(storage).map(|total| LiquidityPool {
            config,
            bank,
            total,
        })
    }
}

impl<Lpn, Bank> LiquidityPool<'_, '_, Lpn, Bank>
where
    Lpn: CurrencyDef,
    Bank: BankAccountView,
{
    /// Computation of the remaining deposit capacity depending on the minimum utilization constraint.
    ///
    /// Returns:
    /// - `Ok(None)` -> no minimum utilization constraint is set
    /// - `Ok(Some(amount))` -> the maximum additional deposit allowed (may be zero)
    /// - `Err(_)` -> an error occured while computing the commited balance
    pub fn deposit_capacity(
        &self,
        now: &Timestamp,
        pending_deposit: Coin<Lpn>,
    ) -> Result<Option<Coin<Lpn>>> {
        let min_utilization = self.config.min_utilization();

        if min_utilization.is_zero() {
            Ok(None)
        } else {
            let total_due = self
                .total_due(now)
                .ok_or(ContractError::overflow_total_due(
                    "Calculating pool's total liability",
                    now,
                ))?;

            self.commited_balance(pending_deposit).map(|balance| {
                if self.utilization(balance, total_due) > min_utilization {
                    // a followup from the above true value is (total_due * 100 / min_utilization) > (balance + total_due)
                    SimpleFraction::new(Percent100::MAX.permilles(), min_utilization.permilles())
                        .of(total_due)
                        .map(|res| res - balance - total_due)
                } else {
                    Some(Coin::ZERO)
                }
            })
        }
    }

    pub fn query_lpp_balance(&self, now: &Timestamp) -> Result<LppBalances<Lpn>> {
        let balance = self.uncommited_balance()?;

        let total_principal_due = self.total.total_principal_due();

        self.total
            .total_interest_due_by_now(now)
            .ok_or(ContractError::overflow_total_interest_due_by_now(
                "Calculating total interest due for Pool's balance",
                now,
            ))
            .map(|total_interest_due| LppBalances {
                balance,
                total_principal_due,
                total_interest_due,
            })
    }

    pub fn calculate_price(
        &self,
        now: &Timestamp,
        uncommited_amount: Coin<Lpn>,
    ) -> Result<NTokenPrice<Lpn>> {
        let balance_nlpn = self.balance_nlpn();

        let price = if balance_nlpn.is_zero() {
            ApiConfig::initial_derivative_price()
        } else {
            price::total_of(balance_nlpn).is(self.total_lpn(now, uncommited_amount)?)
        };

        debug_assert!(
            price >= ApiConfig::initial_derivative_price::<Lpn>(),
            "[Lpp] programming error: nlpn price less than initial"
        );

        Ok(price)
    }

    pub fn validate_lease_addr<V>(&self, validator: &V, lease_addr: Addr) -> Result<Addr>
    where
        V: Validator,
    {
        validator
            .check_contract_code(lease_addr, &self.config.lease_code())
            .map_err(ContractError::from)
    }

    pub fn deposit(&mut self, amount: Coin<Lpn>, now: &Timestamp) -> Result<Coin<NLpn>> {
        debug_assert_ne!(Coin::ZERO, amount);

        if self
            .deposit_capacity(now, amount)?
            .is_some_and(|capacity| amount > capacity)
        {
            return Err(ContractError::UtilizationBelowMinimalRates);
        }

        self.calculate_price(now, amount)
            .and_then(|price| {
                price::total(amount, price.inv()).ok_or_else(|| {
                    ContractError::overflow_price_total(
                        "calculating the receipts",
                        amount,
                        price.inv(),
                    )
                })
            })
            .and_then(|receipts| {
                if receipts.is_zero() {
                    Err(ContractError::DepositLessThanAReceipt)
                } else {
                    self.total.deposit(receipts).map(|_| receipts)
                }
            })
    }

    pub fn withdraw_lpn(
        &mut self,
        receipts: Coin<NLpn>,
        pending_withdraw: Coin<Lpn>,
        now: &Timestamp,
    ) -> Result<Coin<Lpn>> {
        debug_assert_ne!(Coin::ZERO, receipts);

        // the price calculation should go before the withdrawal from the total
        self.calculate_price(now, pending_withdraw)
            .and_then(|price| {
                price::total(receipts, price).ok_or_else(|| {
                    ContractError::overflow_price_total(
                        "calculating the withdrawal amount",
                        receipts,
                        price,
                    )
                })
            })
            .and_then(|amount_lpn: Coin<Lpn>| {
                debug_assert_ne!(
                    Coin::ZERO,
                    amount_lpn,
                    "The receipts price should always be greater to 1!"
                );
                self.total.withdraw(receipts).map(|_| amount_lpn)
            })
            .and_then(|amount_lpn| {
                self.commited_balance(pending_withdraw).and_then(|balance| {
                    if balance < amount_lpn {
                        Err(ContractError::NoLiquidity {})
                    } else {
                        Ok(amount_lpn)
                    }
                })
            })
    }

    pub const fn balance_nlpn(&self) -> Coin<NLpn> {
        self.total.receipts()
    }

    pub fn query_quote(&self, quote: Coin<Lpn>, now: &Timestamp) -> Result<Option<Percent100>> {
        let balance = self.uncommited_balance()?;

        if quote > balance {
            return Ok(None);
        }

        let total_principal_due = self.total.total_principal_due();
        let total_interest = self.total.total_interest_due_by_now(now).ok_or(
            ContractError::overflow_total_interest_due_by_now("Calculating quote interest", now),
        )?;
        let total_liability_past_quote = total_principal_due + quote + total_interest;
        let total_balance_past_quote = balance - quote;

        Ok(Some(self.config.borrow_rate().calculate(
            total_liability_past_quote,
            total_balance_past_quote,
        )))
    }

    pub(super) fn try_open_loan(&mut self, now: Timestamp, amount: Coin<Lpn>) -> Result<Loan<Lpn>> {
        if amount.is_zero() {
            return Err(ContractError::ZeroLoanAmount);
        }

        self.query_quote(amount, &now)
            .and_then(|quote| quote.ok_or(ContractError::NoLiquidity {}))
            .and_then(|annual_interest_rate| {
                self.total
                    .borrow(now, amount, annual_interest_rate)
                    .map(|_total| Loan {
                        principal_due: amount,
                        annual_interest_rate,
                        interest_paid: now,
                    })
            })
    }

    pub(super) fn register_repay_loan(
        &mut self,
        now: Timestamp,
        loan: &Loan<Lpn>,
        payment: &RepayShares<Lpn>,
    ) -> Option<()> {
        self.total.repay(
            now,
            payment.interest,
            payment.principal,
            loan.annual_interest_rate,
        )
    }

    fn uncommited_balance(&self) -> Result<Coin<Lpn>> {
        self.bank.balance().map_err(Into::into)
    }

    fn commited_balance(&self, uncommited_amount: Coin<Lpn>) -> Result<Coin<Lpn>> {
        self.uncommited_balance().map(|balance| {
            debug_assert!(
                uncommited_amount <= balance,
                "Pending deposit or withdraw {{{uncommited_amount:?}}} > Current Balance: {{{balance}}}!"
            );
            balance - uncommited_amount
        })
    }

    fn total_due(&self, now: &Timestamp) -> Option<Coin<Lpn>> {
        self.total
            .total_interest_due_by_now(now)
            .map(|interest| interest + self.total.total_principal_due())
    }

    fn total_lpn(&self, now: &Timestamp, uncommited_amount: Coin<Lpn>) -> Result<Coin<Lpn>> {
        self.total_due(now)
            .ok_or(ContractError::overflow_total_due(
                "Total due overflow while calculating total pool value ",
                now,
            ))
            .and_then(|due| {
                self.commited_balance(uncommited_amount)
                    .map(|balance| due + balance)
            })
    }

    fn utilization(&self, balance: Coin<Lpn>, total_due: Coin<Lpn>) -> Percent100 {
        if balance.is_zero() {
            Percent100::MAX
        } else {
            Percent100::from_ratio(total_due, total_due + balance)
        }
    }
}

#[cfg(test)]
mod test {
    use finance::{
        coin::Coin,
        duration::Duration,
        fraction::Fraction,
        percent::Percent100,
        price::{self, Price},
        zero::Zero,
    };
    use lpp_platform::NLpn;
    use platform::{bank::testing::MockBankView, contract::Code};
    use sdk::cosmwasm_std::{Addr, Timestamp, testing::MockStorage};

    use crate::{
        borrow::InterestRate,
        config::Config as ApiConfig,
        contract::{
            ContractError,
            test::{self, TheCurrency},
        },
        loan::Loan,
        loans::Repo,
        lpp::LppBalances,
    };

    use super::LiquidityPool;

    const BASE_INTEREST_RATE: Percent100 = Percent100::from_permille(70);
    const UTILIZATION_OPTIMAL: Percent100 = Percent100::from_permille(700);
    const ADDON_OPTIMAL_INTEREST_RATE: Percent100 = Percent100::from_permille(20);
    const DEFAULT_MIN_UTILIZATION: Percent100 = Percent100::ZERO;

    #[test]
    fn new_store_load() {
        let bank = MockBankView::<TheCurrency, TheCurrency>::only_balance(Coin::ZERO);
        let lease_code_id = Code::unchecked(123);
        let config = ApiConfig::new(
            lease_code_id,
            InterestRate::new(
                BASE_INTEREST_RATE,
                UTILIZATION_OPTIMAL,
                ADDON_OPTIMAL_INTEREST_RATE,
            )
            .expect("Couldn't construct interest rate value!"),
            DEFAULT_MIN_UTILIZATION,
        );
        let lpp = LiquidityPool::<'_, '_, TheCurrency, _>::new(&config, &bank);

        let mut store = MockStorage::new();
        let now = Timestamp::default();
        lpp.save(&mut store).unwrap();
        let lpp = LiquidityPool::<'_, '_, TheCurrency, _>::load(&store, &config, &bank).unwrap();
        assert_eq!(
            Price::identity(),
            lpp.calculate_price(&now, Coin::ZERO).unwrap()
        );

        assert_eq!(None, lpp.deposit_capacity(&now, Coin::ZERO,).unwrap());
        assert_eq!(
            LppBalances {
                balance: Coin::ZERO,
                total_principal_due: Coin::ZERO,
                total_interest_due: Coin::ZERO
            },
            lpp.query_lpp_balance(&now).unwrap()
        );
    }

    #[test]
    fn test_balance() {
        let lease_code_id = Code::unchecked(123);
        let balance = test::lpn_coin(10_000_000);

        let config = ApiConfig::new(
            lease_code_id,
            InterestRate::new(
                BASE_INTEREST_RATE,
                UTILIZATION_OPTIMAL,
                ADDON_OPTIMAL_INTEREST_RATE,
            )
            .expect("Couldn't construct interest rate value!"),
            DEFAULT_MIN_UTILIZATION,
        );
        let bank = MockBankView::<TheCurrency, TheCurrency>::only_balance(balance);
        let lpp = LiquidityPool::<TheCurrency, _>::new(&config, &bank);

        let balance_lpp = lpp.uncommited_balance().expect("can't get balance");

        assert_eq!(balance_lpp, balance);
    }

    #[test]
    fn test_query_quote() {
        const BALANCE: Coin<TheCurrency> = test::lpn_coin(10_000_000);
        const DEPOSIT_AMOUNT: Coin<TheCurrency> = test::lpn_coin(7_000_000);
        let mut store = MockStorage::default();

        let now = Timestamp::from_nanos(0);

        let lease_code_id = Code::unchecked(123);

        let bank = MockBankView::<_, TheCurrency>::only_balance(BALANCE);
        let config = ApiConfig::new(
            lease_code_id,
            InterestRate::new(
                BASE_INTEREST_RATE,
                UTILIZATION_OPTIMAL,
                ADDON_OPTIMAL_INTEREST_RATE,
            )
            .expect("Couldn't construct interest rate value!"),
            DEFAULT_MIN_UTILIZATION,
        );
        let mut lpp = LiquidityPool::<TheCurrency, _>::new(&config, &bank);

        let now = now + Duration::from_nanos(10); //deliberately hide the variable name
        assert_eq!(
            Percent100::from_permille(136),
            lpp.query_quote(test::lpn_coin(7_700_000), &now)
                .expect("can't query quote")
                .expect("should return some interest_rate")
        );
        lpp.try_open_loan(now, DEPOSIT_AMOUNT).unwrap();
        lpp.save(&mut store).unwrap();

        // wait for a year
        let now = now + Duration::YEAR;
        let bank_past_loan = MockBankView::<_, TheCurrency>::only_balance(BALANCE - DEPOSIT_AMOUNT);
        let lpp = LiquidityPool::<TheCurrency, _>::load(&store, &config, &bank_past_loan).unwrap();

        let result = lpp
            .query_quote(test::lpn_coin(1_000_000), &now)
            .expect("can't query quote")
            .expect("should return some interest_rate");

        assert_eq!(result, Percent100::from_permille(136));
    }

    #[test]
    fn test_open_and_repay_loan() {
        const LPP_BALANCE: Coin<TheCurrency> = test::lpn_coin(10_000_000);
        const LOAN_AMOUNT: Coin<TheCurrency> = test::lpn_coin(5_000_000);

        let base_rate = BASE_INTEREST_RATE;
        let addon_rate = ADDON_OPTIMAL_INTEREST_RATE;
        let utilization_optimal = UTILIZATION_OPTIMAL;

        let interest_rate = InterestRate::new(base_rate, utilization_optimal, addon_rate).unwrap();
        let annual_interest_rate = interest_rate.calculate(LOAN_AMOUNT, LPP_BALANCE - LOAN_AMOUNT);

        let mut store = MockStorage::new();
        let bank = MockBankView::<TheCurrency, TheCurrency>::only_balance(LPP_BALANCE);
        let lease_addr = Addr::unchecked("loan");
        let now = Timestamp::from_nanos(0);
        let lease_code_id = Code::unchecked(123);

        let config = ApiConfig::new(lease_code_id, interest_rate, DEFAULT_MIN_UTILIZATION);

        let mut lpp = LiquidityPool::<TheCurrency, _>::new(&config, &bank);

        let now = now + Duration::from_nanos(10);

        lpp.try_open_loan(now, LOAN_AMOUNT)
            .and_then(|loan| Repo::open(&mut store, lease_addr.clone(), &loan))
            .unwrap();

        let mut loan = Repo::query(&store, lease_addr.clone()).unwrap().unwrap();

        assert_eq!(
            Loan {
                principal_due: LOAN_AMOUNT,
                annual_interest_rate,
                interest_paid: now
            },
            loan
        );
        assert_eq!(loan.interest_due(&now), Some(Coin::ZERO));

        // wait for 36 days
        let now = now + Duration::from_days(36);

        // pay interest for 36 days
        let payment = loan.interest_due(&now).unwrap();

        let repay = loan.repay(&now, payment).unwrap();
        let registration = lpp.register_repay_loan(now, &loan, &repay);
        assert_eq!(registration, Some(()));
        Repo::save(&mut store, lease_addr.clone(), &loan).unwrap();

        assert_eq!(Coin::ZERO, repay.excess);

        assert_eq!(
            Loan {
                principal_due: LOAN_AMOUNT,
                annual_interest_rate,
                interest_paid: now
            },
            loan
        );
        assert_eq!(
            loan,
            Repo::query(&store, lease_addr.clone()).unwrap().unwrap()
        );
        assert_eq!(loan.interest_due(&now), Some(Coin::ZERO));

        // an immediate repay after repay should pass (loan_interest_due==0 bug)
        let repay = loan.repay(&now, Coin::ZERO).unwrap();
        let registration1 = lpp.register_repay_loan(now, &loan, &repay);
        assert_eq!(registration1, Some(()));

        // wait for another 36 days
        let now = now + Duration::from_days(36);

        const PAYED_EXTRA: Coin<TheCurrency> = test::lpn_coin(100);
        // pay everything + excess

        let repay = loan
            .interest_due(&now)
            .and_then(|due| loan.repay(&now, due + LOAN_AMOUNT + PAYED_EXTRA))
            .unwrap();

        let registration2 = lpp.register_repay_loan(now, &loan, &repay);
        assert_eq!(registration2, Some(()));

        assert_eq!(PAYED_EXTRA, repay.excess);
    }

    #[test]
    fn try_open_loan_with_no_liquidity() {
        let now = Timestamp::from_nanos(0);
        let bank = MockBankView::<TheCurrency, TheCurrency>::only_balance(Coin::ZERO);
        let lease_code_id = Code::unchecked(123);

        let config = ApiConfig::new(
            lease_code_id,
            InterestRate::new(
                BASE_INTEREST_RATE,
                UTILIZATION_OPTIMAL,
                ADDON_OPTIMAL_INTEREST_RATE,
            )
            .expect("Couldn't construct interest rate value!"),
            DEFAULT_MIN_UTILIZATION,
        );
        let mut lpp = LiquidityPool::new(&config, &bank);

        let result = lpp.try_open_loan(now, test::lpn_coin(1_000));
        assert_eq!(result, Err(ContractError::NoLiquidity {}));
    }

    #[test]
    fn try_open_loan_for_zero_amount() {
        const BALANCE: Coin<TheCurrency> = test::lpn_coin(10_000_000);
        let now = Timestamp::from_nanos(0);
        let bank = MockBankView::<TheCurrency, TheCurrency>::only_balance(BALANCE);
        let lease_code_id = Code::unchecked(123);

        let config = ApiConfig::new(
            lease_code_id,
            InterestRate::new(
                BASE_INTEREST_RATE,
                UTILIZATION_OPTIMAL,
                ADDON_OPTIMAL_INTEREST_RATE,
            )
            .expect("Couldn't construct interest rate value!"),
            DEFAULT_MIN_UTILIZATION,
        );
        let mut lpp = LiquidityPool::new(&config, &bank);

        let result = lpp.try_open_loan(now, test::lpn_coin(0));
        assert_eq!(result, Err(ContractError::ZeroLoanAmount));
    }

    #[test]
    fn open_loan_repay_zero() {
        const BALANCE: Coin<TheCurrency> = test::lpn_coin(10_000_000);
        let mut store = MockStorage::new();
        let now = Timestamp::from_nanos(0);
        let bank = MockBankView::<TheCurrency, TheCurrency>::only_balance(BALANCE);
        let loan_addr = Addr::unchecked("loan");
        let lease_code_id = Code::unchecked(123);

        let config = ApiConfig::new(
            lease_code_id,
            InterestRate::new(
                BASE_INTEREST_RATE,
                UTILIZATION_OPTIMAL,
                ADDON_OPTIMAL_INTEREST_RATE,
            )
            .expect("Couldn't construct interest rate value!"),
            DEFAULT_MIN_UTILIZATION,
        );
        let mut lpp = LiquidityPool::new(&config, &bank);

        lpp.try_open_loan(now, test::lpn_coin(5_000))
            .and_then(|loan| Repo::open(&mut store, loan_addr.clone(), &loan))
            .unwrap();

        let mut loan_before = Repo::<TheCurrency>::query(&store, loan_addr.clone())
            .unwrap()
            .unwrap();

        //zero repay
        let payment = loan_before.repay(&now, Coin::ZERO).unwrap();
        let registration = lpp.register_repay_loan(now, &loan_before, &payment);
        assert_eq!(registration, Some(()));

        Repo::save(&mut store, loan_addr.clone(), &loan_before).unwrap();

        let loan_after = Repo::query(&store, loan_addr).unwrap().unwrap();

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
        const BALANCE: Coin<TheCurrency> = test::lpn_coin(10_000_000);
        let mut store = MockStorage::new();
        let now = Timestamp::from_nanos(0);
        let bank = MockBankView::<TheCurrency, TheCurrency>::only_balance(BALANCE);
        let loan_addr = Addr::unchecked("loan");
        let lease_code_id = Code::unchecked(123);

        let config = ApiConfig::new(
            lease_code_id,
            InterestRate::new(
                BASE_INTEREST_RATE,
                UTILIZATION_OPTIMAL,
                ADDON_OPTIMAL_INTEREST_RATE,
            )
            .expect("Couldn't construct interest rate value!"),
            DEFAULT_MIN_UTILIZATION,
        );
        let mut lpp = LiquidityPool::new(&config, &bank);

        lpp.try_open_loan(now, test::lpn_coin(5_000))
            .and_then(|loan| Repo::open(&mut store, loan_addr.clone(), &loan))
            .unwrap();

        let mut loan = Repo::<TheCurrency>::query(&store, loan_addr.clone())
            .unwrap()
            .unwrap();
        assert_eq!(Some(Coin::ZERO), loan.interest_due(&now));

        let repay = loan.repay(&now, test::lpn_coin(5_000)).unwrap();
        let registration = lpp.register_repay_loan(now, &loan, &repay);
        assert_eq!(registration, Some(()));

        Repo::save(&mut store, loan_addr.clone(), &loan).unwrap();

        assert_eq!(Coin::ZERO, repay.excess);

        // Should be closed
        assert_eq!(None, Repo::<TheCurrency>::query(&store, loan_addr).unwrap());
    }

    #[test]
    fn test_tvl_and_price() {
        const BALANCE: Coin<TheCurrency> = test::lpn_coin(10_000_000);
        const DEPOSIT: Coin<TheCurrency> = test::lpn_coin(10_000_000);
        const DEPOSIT_RECEIPTS: Coin<NLpn> = Coin::new(10_000_000);
        const LOAN_AMOUNT: Coin<TheCurrency> = test::lpn_coin(5_000_000);
        const EXPECTED_INTEREST_RATE: Percent100 = Percent100::from_permille(220);
        const LOAN_REPAYMENT: Coin<TheCurrency> = test::lpn_coin(6_000_000);

        let mut store = MockStorage::new();
        let now = Timestamp::from_nanos(0);
        let loan_addr = Addr::unchecked("loan");
        let lease_code_id = Code::unchecked(123);

        let bank = MockBankView::<TheCurrency, TheCurrency>::only_balance(BALANCE);

        let config = ApiConfig::new(
            lease_code_id,
            InterestRate::new(
                Percent100::from_percent(18),
                Percent100::from_percent(50),
                Percent100::from_percent(2),
            )
            .expect("Couldn't construct interest rate value!"),
            DEFAULT_MIN_UTILIZATION,
        );
        let mut lpp = LiquidityPool::new(&config, &bank);
        assert_eq!(Ok(DEPOSIT_RECEIPTS), lpp.deposit(DEPOSIT, &now));

        let mut loan = {
            assert_eq!(
                Price::identity(),
                lpp.calculate_price(&now, Coin::ZERO).unwrap()
            );

            assert_eq!(
                EXPECTED_INTEREST_RATE,
                lpp.query_quote(LOAN_AMOUNT, &now).unwrap().unwrap(),
            );

            lpp.try_open_loan(now, LOAN_AMOUNT)
                .and_then(|loan| Repo::open(&mut store, loan_addr.clone(), &loan).map(|()| loan))
                .unwrap()
        };
        lpp.save(&mut store).unwrap();

        const BALANCE_PAST_LOAN: Coin<TheCurrency> = BALANCE.checked_sub(LOAN_AMOUNT).unwrap();
        let loan_interest = EXPECTED_INTEREST_RATE.of(LOAN_AMOUNT);
        // wait a year
        let now = now + Duration::YEAR;
        {
            let bank = MockBankView::<TheCurrency, TheCurrency>::only_balance(BALANCE_PAST_LOAN);
            let mut lpp = LiquidityPool::load(&store, &config, &bank).unwrap();

            let total_lpn = lpp.total_lpn(&now, Coin::ZERO).unwrap();

            assert_eq!(BALANCE_PAST_LOAN + LOAN_AMOUNT + loan_interest, total_lpn);

            let lpp_balance = lpp.query_lpp_balance(&now).unwrap();
            assert_eq!(
                LppBalances {
                    balance: BALANCE_PAST_LOAN,
                    total_principal_due: LOAN_AMOUNT,
                    total_interest_due: loan_interest
                },
                lpp_balance
            );

            let price = lpp.calculate_price(&now, Coin::ZERO).unwrap();
            assert_eq!(
                price::total_of(DEPOSIT_RECEIPTS).is(lpp_balance.into_total()),
                price,
            );

            let payment = loan.repay(&now, LOAN_REPAYMENT).unwrap();
            Repo::save(&mut store, loan_addr.clone(), &loan).unwrap();
            let registration = lpp.register_repay_loan(now, &loan, &payment);
            assert_eq!(registration, Some(()));

            assert_eq!(payment.excess, Coin::ZERO,);
            lpp.save(&mut store).unwrap();
        }

        const BALANCE_PAST_REPAYMENT: Coin<TheCurrency> =
            BALANCE_PAST_LOAN.checked_add(LOAN_REPAYMENT).unwrap();
        let bank = MockBankView::<TheCurrency, TheCurrency>::only_balance(BALANCE_PAST_REPAYMENT);
        let mut lpp = LiquidityPool::load(&store, &config, &bank).unwrap();

        let lpp_total = BALANCE_PAST_REPAYMENT + LOAN_AMOUNT + loan_interest - LOAN_REPAYMENT;
        assert_eq!(lpp_total, lpp.total_lpn(&now, Coin::ZERO).unwrap(),);

        assert_eq!(
            price::total_of(DEPOSIT_RECEIPTS).is(lpp_total),
            lpp.calculate_price(&now, Coin::ZERO).unwrap()
        );

        let withdraw = lpp
            .withdraw_lpn(Coin::new(1000u128), Coin::ZERO, &now)
            .unwrap();
        assert_eq!(withdraw, test::lpn_coin(1110));
    }

    mod min_utilization {
        use finance::{
            coin::{Amount, Coin},
            percent::Percent100,
            zero::Zero,
        };
        use platform::{bank::testing::MockBankView, contract::Code};
        use sdk::cosmwasm_std::Timestamp;

        use crate::{
            borrow::InterestRate, config::Config as ApiConfig, contract::test, state::Total,
        };

        use super::{super::LiquidityPool, TheCurrency};

        const FIFTY_PERCENT_MIN_UTILIZATION: Percent100 = Percent100::from_permille(500);

        fn test_case(
            borrowed: Amount,
            lpp_balance: Amount,
            min_utilization: Percent100,
            expected_limit: Option<Amount>,
        ) {
            let now = Timestamp::from_seconds(120);
            let mut total: Total<TheCurrency> = Total::new();

            total
                .borrow(now, test::lpn_coin(borrowed), Percent100::ZERO)
                .unwrap();

            let bank =
                MockBankView::<TheCurrency, TheCurrency>::only_balance(Coin::new(lpp_balance));
            let lpp = LiquidityPool::<TheCurrency, _> {
                config: &ApiConfig::new(
                    Code::unchecked(0xDEADC0DE_u64),
                    InterestRate::new(
                        Percent100::ZERO,
                        Percent100::from_permille(500),
                        Percent100::MAX,
                    )
                    .unwrap(),
                    min_utilization,
                ),
                total,
                bank: &bank,
            };

            assert_eq!(
                lpp.deposit_capacity(&now, Coin::ZERO).unwrap(),
                expected_limit.map(Coin::new)
            );
        }

        #[test]
        fn test_deposit_capacity_no_min_util_below_50() {
            test_case(50, 100, Percent100::ZERO, None);
        }

        #[test]
        fn test_deposit_capacity_no_min_util_at_50() {
            test_case(50, 50, Percent100::ZERO, None);
        }

        #[test]
        fn test_deposit_capacity_no_min_util_above_50() {
            test_case(100, 50, Percent100::ZERO, None);
        }

        #[test]
        fn test_deposit_capacity_no_min_util_at_100() {
            test_case(50, 0, Percent100::ZERO, None);
        }

        #[test]
        fn test_deposit_capacity_below_min_util() {
            test_case(
                50,
                100,
                FIFTY_PERCENT_MIN_UTILIZATION,
                Some(Default::default()),
            );
        }

        #[test]
        fn test_deposit_capacity_at_min_util() {
            test_case(
                50,
                50,
                FIFTY_PERCENT_MIN_UTILIZATION,
                Some(Default::default()),
            );
        }

        #[test]
        fn test_deposit_capacity_above_min_util() {
            test_case(100, 50, FIFTY_PERCENT_MIN_UTILIZATION, Some(50));
        }

        #[test]
        fn test_deposit_capacity_at_max_util() {
            test_case(50, 0, FIFTY_PERCENT_MIN_UTILIZATION, Some(50));
        }
    }

    mod lending {
        use finance::{
            coin::Coin,
            duration::Duration,
            percent::Percent100,
            price::{self, Price},
            zero::Zero,
        };
        use lpp_platform::NLpn;
        use platform::{bank::testing::MockBankView, contract::Code};
        use sdk::cosmwasm_std::{Timestamp, testing::MockStorage};

        use crate::{
            borrow::InterestRate,
            config::Config,
            contract::{ContractError, test},
            lpp::{LiquidityPool, test::TheCurrency},
        };

        #[test]
        fn test_deposit() {
            let now = Timestamp::from_seconds(120);
            const DEPOSIT1: Coin<TheCurrency> = test::lpn_coin(1233);
            const RECEIPT1: Coin<NLpn> = Coin::new(1233);
            const DEPOSIT2: Coin<TheCurrency> = test::lpn_coin(3113);
            const LOAN: Coin<TheCurrency> = test::lpn_coin(1000);

            let mut store = MockStorage::default();

            let config = Config::new(
                Code::unchecked(0xDEADC0DE_u64),
                InterestRate::new(
                    Percent100::ZERO,
                    Percent100::from_permille(500),
                    Percent100::from_permille(500),
                )
                .unwrap(),
                Percent100::ZERO,
            );
            let bank = MockBankView::<TheCurrency, TheCurrency>::only_balance(DEPOSIT1);
            let mut lpp = LiquidityPool::<TheCurrency, _>::new(&config, &bank);

            assert_eq!(RECEIPT1, lpp.deposit(DEPOSIT1, &now).unwrap());
            assert_eq!(RECEIPT1, lpp.balance_nlpn());

            assert_eq!(
                ContractError::NoLiquidity {},
                lpp.try_open_loan(now, DEPOSIT1 + Coin::new(1)).unwrap_err()
            );

            lpp.try_open_loan(now, LOAN)
                .inspect(|loan| assert_eq!(LOAN, loan.principal_due))
                .unwrap();
            lpp.save(&mut store).unwrap();

            // let's see how the due interest affects the deposited coins
            let now = now + Duration::from_days(120);
            let bank =
                MockBankView::<TheCurrency, TheCurrency>::only_balance(DEPOSIT1 - LOAN + DEPOSIT2);
            let mut lpp = LiquidityPool::<TheCurrency, _>::load(&store, &config, &bank).unwrap();

            let nlpn_to_lpn_before = lpp.calculate_price(&now, DEPOSIT2).unwrap();
            assert!(nlpn_to_lpn_before > Price::identity());
            let expected_receipt2 = price::total(DEPOSIT2, nlpn_to_lpn_before.inv()).unwrap();
            assert_eq!(expected_receipt2, lpp.deposit(DEPOSIT2, &now).unwrap());
            assert_eq!(RECEIPT1 + expected_receipt2, lpp.balance_nlpn());
        }

        #[test]
        fn test_withdraw() {
            let now = Timestamp::from_seconds(120);
            const DEPOSIT1: Coin<TheCurrency> = test::lpn_coin(1233);
            const RECEIPT1: Coin<NLpn> = Coin::new(1233);
            const WITHDRAW1: Coin<NLpn> = Coin::new(123);
            const LOAN: Coin<TheCurrency> = test::lpn_coin(1000);

            let mut store = MockStorage::default();

            let config = Config::new(
                Code::unchecked(0xDEADC0DE_u64),
                InterestRate::new(
                    Percent100::ZERO,
                    Percent100::from_permille(500),
                    Percent100::from_permille(500),
                )
                .unwrap(),
                Percent100::ZERO,
            );
            let bank = MockBankView::<TheCurrency, TheCurrency>::only_balance(DEPOSIT1);
            let mut lpp = LiquidityPool::<TheCurrency, _>::new(&config, &bank);

            assert!(matches!(
                lpp.withdraw_lpn(RECEIPT1, Coin::ZERO, &now).unwrap_err(),
                ContractError::ComputationOverflow { .. }
            ));

            assert_eq!(RECEIPT1, lpp.deposit(DEPOSIT1, &now).unwrap());
            assert_eq!(RECEIPT1, lpp.balance_nlpn());

            lpp.try_open_loan(now, LOAN)
                .inspect(|loan| assert_eq!(LOAN, loan.principal_due))
                .unwrap();
            lpp.save(&mut store).unwrap();

            // let's see how the due interest affects the withdrawn coins
            let now = now + Duration::from_days(120);
            let bank = MockBankView::<TheCurrency, TheCurrency>::only_balance(DEPOSIT1 - LOAN);
            let mut lpp = LiquidityPool::<TheCurrency, _>::load(&store, &config, &bank).unwrap();

            let nlpn_to_lpn_before = lpp.calculate_price(&now, Coin::ZERO).unwrap();
            assert!(nlpn_to_lpn_before > Price::identity());
            let expected_withdraw1 = price::total(WITHDRAW1, nlpn_to_lpn_before).unwrap();
            assert_eq!(
                expected_withdraw1,
                lpp.withdraw_lpn(WITHDRAW1, Coin::ZERO, &now).unwrap()
            );

            assert_eq!(RECEIPT1 - WITHDRAW1, lpp.balance_nlpn());
        }

        #[test]
        fn test_deposit_less_than_a_receipt() {
            let now = Timestamp::from_seconds(120);
            const DEPOSIT1: Coin<TheCurrency> = test::lpn_coin(1233);
            const RECEIPT1: Coin<NLpn> = Coin::new(1233);
            const INTEREST: Coin<TheCurrency> = test::lpn_coin(1);
            const DEPOSIT2: Coin<TheCurrency> = test::lpn_coin(1); // the receipts would be 1233/1234 that should trigger an error

            let mut store = MockStorage::default();

            let config = Config::new(
                Code::unchecked(0xDEADC0DE_u64),
                InterestRate::new(
                    Percent100::ZERO,
                    Percent100::from_permille(500),
                    Percent100::MAX,
                )
                .unwrap(),
                Percent100::ZERO,
            );
            let bank = MockBankView::<TheCurrency, TheCurrency>::only_balance(DEPOSIT1);
            let mut lpp = LiquidityPool::<TheCurrency, _>::new(&config, &bank);

            assert_eq!(RECEIPT1, lpp.deposit(DEPOSIT1, &now).unwrap());
            assert_eq!(RECEIPT1, lpp.balance_nlpn());
            lpp.save(&mut store).unwrap();

            let bank = MockBankView::<TheCurrency, TheCurrency>::only_balance(
                DEPOSIT1 + INTEREST + DEPOSIT2,
            );
            let mut lpp = LiquidityPool::<TheCurrency, _>::load(&store, &config, &bank).unwrap();
            assert_eq!(
                ContractError::DepositLessThanAReceipt,
                lpp.deposit(DEPOSIT2, &now).unwrap_err()
            );
        }
    }
}
