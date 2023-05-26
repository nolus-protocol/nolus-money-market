use std::cmp;

use cosmwasm_std::Storage;
use sdk::{
    cosmwasm_std::{Addr, Timestamp},
    cw_storage_plus::Map,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use finance::{
    coin::Coin, currency::Currency, duration::Duration, interest::InterestPeriod, percent::Percent,
};
use sdk::schemars::{self, JsonSchema};

use crate::error::{ContractError, Result};

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Eq, PartialEq))]
#[serde(rename_all = "snake_case")]
pub struct Loan<Lpn>
where
    Lpn: Currency,
{
    pub principal_due: Coin<Lpn>,
    pub annual_interest_rate: Percent,
    pub interest_paid: Timestamp,
}

pub struct RepayShares<LPN>
where
    LPN: Currency,
{
    pub interest: Coin<LPN>,
    pub principal: Coin<LPN>,
    pub excess: Coin<LPN>,
}

impl<Lpn> Loan<Lpn>
where
    Lpn: Currency,
{
    const STORAGE: Map<'static, Addr, Loan<Lpn>> = Map::new("loans");

    pub fn interest_due(&self, by: Timestamp) -> Coin<Lpn> {
        let delta_t = Duration::between(self.interest_paid, cmp::max(by, self.interest_paid));

        let interest_period = InterestPeriod::with_interest(self.annual_interest_rate)
            .from(self.interest_paid)
            .spanning(delta_t);

        interest_period.interest(self.principal_due)
    }

    pub fn repay(&mut self, by: Timestamp, repayment: Coin<Lpn>) -> RepayShares<Lpn> {
        let (due_period, interest_change) =
            InterestPeriod::with_interest(self.annual_interest_rate)
                .from(self.interest_paid)
                .spanning(Duration::between(self.interest_paid, by))
                .pay(self.principal_due, repayment, by);

        let interest_paid = repayment - interest_change;
        let principal_paid = cmp::min(interest_change, self.principal_due);
        let excess = interest_change - principal_paid;

        self.principal_due -= principal_paid;
        self.interest_paid = due_period.start();

        RepayShares {
            interest: interest_paid,
            principal: principal_paid,
            excess,
        }
    }
}

impl<Lpn> Loan<Lpn>
where
    Lpn: Currency + Serialize + DeserializeOwned,
{
    pub fn open(storage: &mut dyn Storage, addr: Addr, loan: &Self) -> Result<()> {
        if Self::STORAGE.has(storage, addr.clone()) {
            return Err(ContractError::LoanExists {});
        }

        Self::STORAGE.save(storage, addr, loan).map_err(Into::into)
    }

    pub fn load(storage: &dyn Storage, addr: Addr) -> Result<Self> {
        Self::STORAGE.load(storage, addr).map_err(Into::into)
    }

    pub fn save(storage: &mut dyn Storage, addr: Addr, loan: Self) -> Result<()> {
        if loan.principal_due.is_zero() {
            Self::STORAGE.remove(storage, addr);
            Ok(())
        } else {
            Self::STORAGE
                .update(storage, addr, |loaded_loan| {
                    let mut loaded_loan = loaded_loan.ok_or(ContractError::NoLoan {})?;
                    loaded_loan.principal_due = loan.principal_due;
                    loaded_loan.interest_paid = loan.interest_paid;

                    Ok::<_, ContractError>(loaded_loan)
                })
                .map(|_| ())
        }
    }

    pub fn query(storage: &dyn Storage, lease_addr: Addr) -> Result<Option<Loan<Lpn>>> {
        Self::STORAGE
            .may_load(storage, lease_addr)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod test {
    use finance::{
        coin::Coin, duration::Duration, percent::Percent, test::currency::Usdc, zero::Zero,
    };
    use sdk::cosmwasm_std::Timestamp;

    use crate::loan::Loan;

    #[test]
    fn interest() {
        let l = Loan {
            principal_due: Coin::<Usdc>::from(100),
            annual_interest_rate: Percent::from_percent(50),
            interest_paid: Timestamp::from_nanos(200),
        };

        assert_eq!(
            Coin::<Usdc>::from(50),
            l.interest_due(l.interest_paid + Duration::YEAR)
        );

        assert_eq!(Coin::ZERO, l.interest_due(l.interest_paid));
        assert_eq!(Coin::ZERO, l.interest_due(l.interest_paid.minus_nanos(1)));
    }

    mod persistence {
        use finance::{
            coin::Coin, duration::Duration, percent::Percent, test::currency::Usdc, zero::Zero,
        };
        use sdk::cosmwasm_std::{testing, Addr, Timestamp};

        use crate::{error::ContractError, loan::Loan};

        #[test]
        fn test_open_and_repay_loan() {
            let mut deps = testing::mock_dependencies();

            let mut time = Timestamp::from_nanos(0);

            let addr = Addr::unchecked("leaser");
            let loan = Loan {
                principal_due: Coin::<Usdc>::new(1000),
                annual_interest_rate: Percent::from_percent(20),
                interest_paid: time,
            };
            Loan::open(deps.as_mut().storage, addr.clone(), &loan).expect("should open loan");

            let result = Loan::open(deps.as_mut().storage, addr.clone(), &loan);
            assert_eq!(result, Err(ContractError::LoanExists {}));

            let mut loan: Loan<Usdc> =
                Loan::load(deps.as_ref().storage, addr.clone()).expect("should load loan");

            time = Timestamp::from_nanos(Duration::YEAR.nanos() / 2);
            let interest: Coin<Usdc> = loan.interest_due(time);
            assert_eq!(interest, 100u128.into());

            // partial repay
            let payment = loan.repay(time, 600u128.into());
            assert_eq!(payment.interest, 100u128.into());
            assert_eq!(payment.principal, 500u128.into());
            assert_eq!(payment.excess, 0u128.into());

            assert_eq!(loan.principal_due, 500u128.into());
            Loan::save(deps.as_mut().storage, addr.clone(), loan).unwrap();

            let mut loan: Loan<Usdc> =
                Loan::load(deps.as_ref().storage, addr.clone()).expect("should load loan");

            // repay with excess, should close the loan
            let payment = loan.repay(time, 600u128.into());
            assert_eq!(payment.interest, 0u128.into());
            assert_eq!(payment.principal, 500u128.into());
            assert_eq!(payment.excess, 100u128.into());
            assert_eq!(loan.principal_due, Coin::ZERO);
            Loan::save(deps.as_mut().storage, addr.clone(), loan).unwrap();

            // is it cleaned up?
            let is_none = Loan::<Usdc>::query(deps.as_ref().storage, addr)
                .expect("should query loan")
                .is_none();
            assert!(is_none);
        }
    }
}
