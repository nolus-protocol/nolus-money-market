use serde::{Deserialize, Serialize};

use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    fraction::Fraction,
    interest,
    percent::Percent,
    ratio::Rational,
    zero::Zero,
};
use lpp_platform::NLpn;
use sdk::{
    cosmwasm_std::{Storage, Timestamp},
    cw_storage_plus::Item,
};

use crate::contract::{ContractError, Result as ContractResult};

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq,))]
#[serde(bound(serialize = "", deserialize = ""))]
pub struct Total<Lpn> {
    /// The total due principle amount
    ///
    /// It is a sum of all loan due principle amounts and is maintained
    /// on loan open and payments.
    total_principal_due: Coin<Lpn>,

    /// Estimation of the total due interest accrued up to `last_update_time`.
    ///
    /// The most precision calculation would be to sum all loan due interest amounts up to that time.
    /// Since there might be a lot of open loans, we may not afford it on chain.
    /// The algorithm keeps a current pool-wide `annual_interest_rate`. It is a weighted average
    /// of all loan interest rates and is updated with each change of the total prindipal due.
    total_interest_due: Coin<Lpn>,

    /// Current pool-wide weghted annual interest rate of all loans interest rates
    annual_interest_rate: Rational<Coin<Lpn>>,

    /// The last time a borrow-related operation is performed
    ///
    /// This concerns only loan open and payments.
    last_update_time: Timestamp,

    /// The total receipts issued to lenders for their deposits
    receipts: Coin<NLpn>,
}

pub mod migrate_from_0_8_12 {
    use serde::{Deserialize, Serialize, Serializer};

    use finance::{coin::Coin, ratio::Rational};
    use lpp_platform::NLpn;
    use sdk::{
        cosmwasm_std::{Storage, Timestamp},
        cw_storage_plus::Item,
    };

    use crate::{contract::Result as ContractResult, state::Total};

    pub fn migrate<Lpn>(store: &mut dyn Storage, balance_nlpn: Coin<NLpn>) -> ContractResult<()> {
        #[derive(Deserialize)]
        #[serde(bound(deserialize = ""))]
        pub struct Total0_8_12<Lpn> {
            total_principal_due: Coin<Lpn>,
            total_interest_due: Coin<Lpn>,
            annual_interest_rate: Rational<Coin<Lpn>>,
            last_update_time: Timestamp,
        }
        impl<Lpn> Serialize for Total0_8_12<Lpn> {
            fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                unimplemented!("satisfies Item::load trait bound")
            }
        }

        let old_key = Item::new("total");
        old_key
            .load(store)
            .inspect(|_| old_key.remove(store))
            .map_err(Into::into)
            .map(|old_total: Total0_8_12<Lpn>| Total::<Lpn> {
                total_principal_due: old_total.total_principal_due,
                total_interest_due: old_total.total_interest_due,
                annual_interest_rate: old_total.annual_interest_rate,
                last_update_time: old_total.last_update_time,
                receipts: balance_nlpn,
            })
            .and_then(|new_total| new_total.store(store))
    }
}

impl<Lpn> Default for Total<Lpn> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Lpn> Total<Lpn> {
    const STORAGE: Item<Total<Lpn>> = Item::new("total");

    pub fn new() -> Self {
        Total {
            total_principal_due: Coin::ZERO,
            total_interest_due: Coin::ZERO,
            annual_interest_rate: zero_interest_rate(),
            last_update_time: Timestamp::default(),
            receipts: Coin::ZERO,
        }
    }

    pub fn store(&self, storage: &mut dyn Storage) -> ContractResult<()> {
        Self::STORAGE.save(storage, self).map_err(Into::into)
    }

    pub fn load(storage: &dyn Storage) -> ContractResult<Self> {
        Self::STORAGE.load(storage).map_err(Into::into)
    }

    pub fn total_principal_due(&self) -> Coin<Lpn> {
        self.total_principal_due
    }

    pub fn total_interest_due_by_now(&self, ctime: &Timestamp) -> Coin<Lpn> {
        if self.total_principal_due.is_zero() {
            // TODO remove this case once close protocols with `total_principal_due == 0` and `total_interest_due > 0`
            // the newly added invariant: if `total_principal_due == 0` then `total_interest_due == 0`
            Coin::ZERO
            //debug_assert!(self.total_interest_due.is_zero());
        } else {
            interest::interest::<Coin<Lpn>, _, _>(
                self.annual_interest_rate,
                self.total_principal_due,
                Duration::between(&self.last_update_time, ctime),
            ) + self.total_interest_due
        }
    }

    pub fn receipts(&self) -> Coin<NLpn> {
        self.receipts
    }

    pub fn borrow(
        &mut self,
        ctime: Timestamp,
        amount: Coin<Lpn>,
        loan_interest_rate: Percent,
    ) -> Result<&Self, ContractError> {
        self.total_interest_due = self.total_interest_due_by_now(&ctime);

        let new_total_principal_due = self
            .total_principal_due
            .checked_add(amount)
            .ok_or(ContractError::OverflowError("Total principal due overflow"))?;

        // TODO: get rid of fully qualified syntax
        let new_annual_interest = self
            .estimated_annual_interest()
            .checked_add(loan_interest_rate.of(amount))
            .ok_or(ContractError::OverflowError(
                "Annual interest calculation overflow",
            ))?;

        self.annual_interest_rate = Rational::new(new_annual_interest, new_total_principal_due);

        self.total_principal_due = new_total_principal_due;

        self.last_update_time = ctime;

        Ok(self)
    }

    pub fn repay(
        &mut self,
        ctime: Timestamp,
        loan_interest_payment: Coin<Lpn>,
        loan_principal_payment: Coin<Lpn>,
        loan_interest_rate: Percent,
    ) -> &Self {
        // The interest payment calculation of loans is the source of truth.
        // Therefore, it is possible for the rounded-down total interest due from `total_interest_due_by_now`
        // to become less than the sum of loans' interests. Taking 0 when subtracting a loan's interest from the total is a safe solution.
        let new_total_interest_due = self
            .total_interest_due_by_now(&ctime)
            .saturating_sub(loan_interest_payment);

        let new_total_principal_due = self
            .total_principal_due
            .checked_sub(loan_principal_payment)
            .expect("Unexpected overflow when subtracting loan principal payment from total principal due");

        if new_total_principal_due.is_zero() {
            // Due to rounding errors, the calculated total interest due might deviate from
            // the sum of loans' interest due. This is an important checkpoint at which
            // the deviation could be cleared.
            self.total_interest_due = Coin::ZERO;

            self.annual_interest_rate = zero_interest_rate();
        } else {
            self.total_interest_due = new_total_interest_due;

            // Please refer to the comment above for more detailed information on why using `saturating_sub` is a safe solution
            // for updating the annual interest
            self.annual_interest_rate = Rational::new(
                self.estimated_annual_interest()
                    .saturating_sub(loan_interest_rate.of(loan_principal_payment)),
                new_total_principal_due,
            )
        };

        self.total_principal_due = new_total_principal_due;

        self.last_update_time = ctime;

        self
    }

    pub fn deposit(&mut self, receipts: Coin<NLpn>) -> Result<&mut Self, ContractError> {
        debug_assert_ne!(Coin::ZERO, receipts);

        self.receipts
            .checked_add(receipts)
            .ok_or(ContractError::OverflowError("Deposit receipts overflow"))
            .map(|total| {
                self.receipts = total;
                self
            })
    }

    pub fn withdraw(&mut self, receipts: Coin<NLpn>) -> Result<&mut Self, ContractError> {
        debug_assert_ne!(Coin::ZERO, receipts);

        self.receipts
            .checked_sub(receipts)
            .ok_or(ContractError::OverflowError("Withdraw receipts overflow"))
            .map(|total| {
                self.receipts = total;
                self
            })
    }

    fn estimated_annual_interest(&self) -> Coin<Lpn> {
        Fraction::<Coin<Lpn>>::of(&self.annual_interest_rate, self.total_principal_due)
    }
}

fn zero_interest_rate<Lpn>() -> Rational<Coin<Lpn>> {
    const THOUSAND: Amount = 1000;
    Rational::new(Coin::ZERO, THOUSAND.into())
}

#[cfg(test)]
mod test {
    use currencies::Lpn;
    use finance::duration::Duration;
    use sdk::cosmwasm_std::testing::MockStorage;

    use crate::loan::Loan;

    use super::*;

    #[test]
    fn borrow_and_repay() {
        let mut store = MockStorage::default();
        let mut block_time = Timestamp::from_nanos(1_571_797_419_879_305_533);

        let total: Total<Lpn> = Total::default();
        total.store(&mut store).expect("should store");

        let mut total: Total<Lpn> = Total::load(&store).expect("should load");

        assert_eq!(Total::default(), total);
        assert_eq!(Coin::ZERO, total.total_principal_due());

        total
            .borrow(block_time, Coin::new(10000), Percent::from_percent(20))
            .unwrap();
        assert_eq!(total.total_principal_due(), Coin::new(10000));

        block_time = block_time.plus_nanos(Duration::YEAR.nanos() / 2);
        let interest_due = total.total_interest_due_by_now(&block_time);
        assert_eq!(interest_due, Coin::new(1000));

        total.repay(
            block_time,
            Coin::new(1000),
            Coin::new(5000),
            Percent::from_percent(20),
        );
        assert_eq!(total.total_principal_due(), Coin::new(5000));

        block_time = block_time.plus_nanos(Duration::YEAR.nanos() / 2);
        let interest_due = total.total_interest_due_by_now(&block_time);
        assert_eq!(interest_due, 500u128.into());
    }

    #[test]
    fn borrow_and_repay_with_overflow() {
        let mut block_time = Timestamp::from_nanos(0);

        let mut total: Total<Lpn> = Total::default();
        assert_eq!(total.total_principal_due(), Coin::<Lpn>::new(0));

        let borrow_loan1 = Coin::<Lpn>::new(5_458_329);
        let loan1_annual_interest_rate = Percent::from_permille(137);
        let loan1 = Loan {
            principal_due: borrow_loan1,
            annual_interest_rate: loan1_annual_interest_rate,
            interest_paid: block_time,
        };

        total
            .borrow(block_time, borrow_loan1, loan1_annual_interest_rate)
            .unwrap();
        assert_eq!(total.total_principal_due(), borrow_loan1);
        assert_eq!(total.total_interest_due_by_now(&block_time), Coin::ZERO);

        block_time = block_time.plus_days(59);

        // Open loan2 after 59 days
        let borrow_loan2 = Coin::<Lpn>::new(3_543_118);
        let loan2_annual_interest_rate = Percent::from_permille(133);
        let loan2 = Loan {
            principal_due: borrow_loan2,
            annual_interest_rate: loan2_annual_interest_rate,
            interest_paid: block_time,
        };

        let total_interest_due = total.total_interest_due_by_now(&block_time);
        assert_eq!(total_interest_due, loan1.interest_due(&block_time));

        total
            .borrow(block_time, borrow_loan2, loan2_annual_interest_rate)
            .unwrap();
        assert_eq!(total.total_principal_due(), borrow_loan1 + borrow_loan2);
        assert_eq!(
            total.total_interest_due_by_now(&block_time),
            total_interest_due
        );

        block_time = block_time.plus_days(147);

        // Fully repay loan1 after 147 days
        total.repay(
            block_time,
            loan1.interest_due(&block_time),
            loan1.principal_due,
            loan1.annual_interest_rate,
        );
        assert_eq!(total.total_principal_due(), borrow_loan2);

        block_time = block_time.plus_days(67);

        // Fully repay loan2 after 67 days
        total.repay(
            block_time,
            loan2.interest_due(&block_time),
            loan2.principal_due,
            loan2.annual_interest_rate,
        );

        assert!(total.total_interest_due.is_zero());
        assert!(total.total_principal_due.is_zero());
    }

    #[test]
    fn deposit() {
        assert_eq!(Coin::ZERO, Total::<Lpn>::default().receipts());

        const RECEIPTS1: Coin<NLpn> = Coin::new(10);
        const RECEIPTS2: Coin<NLpn> = Coin::new(20);
        assert_eq!(
            RECEIPTS1,
            Total::<Lpn>::default()
                .deposit(RECEIPTS1)
                .unwrap()
                .receipts()
        );
        assert!(matches!(
            Total::<Lpn>::default()
                .deposit(RECEIPTS1)
                .unwrap()
                .deposit(Coin::new(Amount::MAX))
                .unwrap_err(),
            ContractError::OverflowError(_)
        ));
        assert_eq!(
            Total::<Lpn>::default()
                .deposit(RECEIPTS1 + RECEIPTS2)
                .unwrap(),
            Total::default()
                .deposit(RECEIPTS1)
                .unwrap()
                .deposit(RECEIPTS2)
                .unwrap()
        );
        assert_eq!(
            RECEIPTS1 + RECEIPTS2,
            Total::<Lpn>::default()
                .deposit(RECEIPTS1)
                .unwrap()
                .deposit(RECEIPTS2)
                .unwrap()
                .receipts()
        );
    }

    #[test]
    fn deposit_persisted() {
        let mut store = MockStorage::default();

        const RECEIPTS1: Coin<NLpn> = Coin::new(10);
        Total::<Lpn>::default()
            .deposit(RECEIPTS1)
            .unwrap()
            .store(&mut store)
            .unwrap();

        let loaded = Total::<Lpn>::load(&store).unwrap();
        assert_eq!(RECEIPTS1, loaded.receipts());
        assert_eq!(*Total::default().deposit(RECEIPTS1).unwrap(), loaded);
    }

    #[test]
    fn withdraw() {
        const RECEIPTS1: Coin<NLpn> = Coin::new(10);
        const RECEIPTS2: Coin<NLpn> = Coin::new(20);
        let mut total: Total<Lpn> = Total::default();
        assert_eq!(
            RECEIPTS1 + RECEIPTS2,
            total.deposit(RECEIPTS1 + RECEIPTS2).unwrap().receipts()
        );

        assert_eq!(RECEIPTS2, total.withdraw(RECEIPTS1).unwrap().receipts(),);
        assert_eq!(Coin::ZERO, total.withdraw(RECEIPTS2).unwrap().receipts(),);
        assert!(matches!(
            total.withdraw(RECEIPTS1).unwrap_err(),
            ContractError::OverflowError(_)
        ));
    }

    #[test]
    fn withdraw_persisted() {
        let mut store = MockStorage::default();

        const RECEIPTS1: Coin<NLpn> = Coin::new(10);
        const RECEIPTS2: Coin<NLpn> = Coin::new(20);
        Total::<Lpn>::default()
            .deposit(RECEIPTS1 + RECEIPTS2)
            .unwrap()
            .withdraw(RECEIPTS1)
            .unwrap()
            .store(&mut store)
            .unwrap();

        let loaded = Total::<Lpn>::load(&store).unwrap();
        assert_eq!(RECEIPTS2, loaded.receipts());
        assert_eq!(*Total::default().deposit(RECEIPTS2).unwrap(), loaded);
    }
}
