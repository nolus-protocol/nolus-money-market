use std::cmp;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use finance::{
    coin::Coin, currency::Currency, duration::Duration, interest::InterestPeriod, percent::Percent,
};
use sdk::{
    cosmwasm_std::{Addr, StdResult, Storage, Timestamp},
    cw_storage_plus::Map,
    schemars::{self, JsonSchema},
};

use crate::error::ContractError;

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Eq, PartialEq))]
#[serde(rename_all = "snake_case")]
pub struct LoanData<Lpn>
where
    Lpn: Currency,
{
    pub principal_due: Coin<Lpn>,
    pub annual_interest_rate: Percent,
    pub interest_paid: Timestamp,
}

impl<Lpn> LoanData<Lpn>
where
    Lpn: Currency,
{
    pub fn interest_due(&self, by: Timestamp) -> Coin<Lpn> {
        let delta_t = Duration::between(self.interest_paid, cmp::max(by, self.interest_paid));

        let interest_period = InterestPeriod::with_interest(self.annual_interest_rate)
            .from(self.interest_paid)
            .spanning(delta_t);

        interest_period.interest(self.principal_due)
    }
}

pub struct Loan<LPN>
where
    LPN: Currency,
{
    addr: Addr,
    data: LoanData<LPN>,
}

pub struct RepayShares<LPN>
where
    LPN: Currency,
{
    pub interest: Coin<LPN>,
    pub principal: Coin<LPN>,
    pub excess: Coin<LPN>,
}

impl<LPN> Loan<LPN>
where
    LPN: Currency + Serialize + DeserializeOwned,
{
    const STORAGE: Map<'static, Addr, LoanData<LPN>> = Map::new("loans");

    pub fn open(
        storage: &mut dyn Storage,
        addr: Addr,
        amount: Coin<LPN>,
        annual_interest_rate: Percent,
        current_time: Timestamp,
    ) -> Result<(), ContractError> {
        if Self::STORAGE.has(storage, addr.clone()) {
            return Err(ContractError::LoanExists {});
        }

        let data = LoanData {
            principal_due: amount,
            annual_interest_rate,
            interest_paid: current_time,
        };

        Self::STORAGE
            .save(storage, addr, &data)
            .map_err(ContractError::Std)
    }

    pub fn load(storage: &dyn Storage, addr: Addr) -> StdResult<Self> {
        let data = Self::STORAGE.load(storage, addr.clone())?;
        let loan = Self { data, addr };

        Ok(loan)
    }

    pub fn data(&self) -> &LoanData<LPN> {
        &self.data
    }

    /// change the Loan state after repay, return (principal_payment, excess_received) pair
    pub fn repay(
        self,
        storage: &mut dyn Storage,
        ctime: Timestamp,
        repay_amount: Coin<LPN>,
    ) -> Result<RepayShares<LPN>, ContractError> {
        let time_delta = Duration::between(self.data.interest_paid, ctime);

        let (interest_period, interest_pay_excess) =
            InterestPeriod::with_interest(self.data.annual_interest_rate)
                .from(self.data.interest_paid)
                .spanning(time_delta)
                .pay(self.data.principal_due, repay_amount, ctime);

        let loan_interest_payment = repay_amount - interest_pay_excess;
        let loan_principal_payment = cmp::min(interest_pay_excess, self.data.principal_due);
        let excess_received = interest_pay_excess - loan_principal_payment;

        if self.data.principal_due == loan_principal_payment {
            Self::STORAGE.remove(storage, self.addr);
        } else {
            Self::STORAGE.update(
                storage,
                self.addr,
                |loan| -> Result<LoanData<LPN>, ContractError> {
                    let mut loan = loan.ok_or(ContractError::NoLoan {})?;
                    loan.principal_due -= loan_principal_payment;
                    loan.interest_paid = interest_period.start();

                    Ok(loan)
                },
            )?;
        }
        Ok(RepayShares {
            interest: loan_interest_payment,
            principal: loan_principal_payment,
            excess: excess_received,
        })
    }

    pub fn query(storage: &dyn Storage, lease_addr: Addr) -> StdResult<Option<LoanData<LPN>>> {
        Self::STORAGE.may_load(storage, lease_addr)
    }
}

#[cfg(test)]
mod test {
    use finance::{duration::Duration, test::currency::Usdc, zero::Zero};
    use sdk::cosmwasm_std::testing;

    use super::*;

    #[test]
    fn test_open_and_repay_loan() {
        let mut deps = testing::mock_dependencies();

        let mut time = Timestamp::from_nanos(0);

        let addr = Addr::unchecked("leaser");
        Loan::open(
            deps.as_mut().storage,
            addr.clone(),
            Coin::<Usdc>::new(1000),
            Percent::from_percent(20),
            time,
        )
        .expect("should open loan");

        let result = Loan::open(
            deps.as_mut().storage,
            addr.clone(),
            Coin::<Usdc>::new(1000),
            Percent::from_percent(20),
            time,
        );
        assert_eq!(result, Err(ContractError::LoanExists {}));

        let loan: Loan<Usdc> =
            Loan::load(deps.as_ref().storage, addr.clone()).expect("should load loan");

        time = Timestamp::from_nanos(Duration::YEAR.nanos() / 2);
        let interest: Coin<Usdc> = loan.data.interest_due(time);
        assert_eq!(interest, 100u128.into());

        // partial repay
        let payment = loan
            .repay(deps.as_mut().storage, time, 600u128.into())
            .expect("should repay");
        assert_eq!(payment.interest, 100u128.into());
        assert_eq!(payment.principal, 500u128.into());
        assert_eq!(payment.excess, 0u128.into());

        let resp = Loan::<Usdc>::query(deps.as_ref().storage, addr.clone())
            .expect("should query loan")
            .expect("should be some loan");

        assert_eq!(resp.principal_due, 500u128.into());

        let loan: Loan<Usdc> =
            Loan::load(deps.as_ref().storage, addr.clone()).expect("should load loan");

        // repay with excess, should close the loan
        let payment = loan
            .repay(deps.as_mut().storage, time, 600u128.into())
            .expect("should repay");
        assert_eq!(payment.interest, 0u128.into());
        assert_eq!(payment.principal, 500u128.into());
        assert_eq!(payment.excess, 100u128.into());

        // is it cleaned up?
        let is_none = Loan::<Usdc>::query(deps.as_ref().storage, addr)
            .expect("should query loan")
            .is_none();
        assert!(is_none);
    }

    #[test]
    fn interest() {
        let l = LoanData {
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
}
